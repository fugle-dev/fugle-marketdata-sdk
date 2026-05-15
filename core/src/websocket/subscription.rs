//! WebSocket subscription state management with ordered preservation

use crate::models::SubscribeRequest;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::RwLock;

/// Manages WebSocket subscription state with insertion order preservation
///
/// Tracks active subscriptions and maintains their order for reconnection.
/// Also tracks server-returned subscription ids so unsubscribe can send the
/// correct id back to the server (Fugle's protocol requires the id it issued
/// in the `subscribed` ack, not a client-side composite key).
///
/// Thread-safe with RwLock for concurrent access.
pub struct SubscriptionManager {
    /// Maps subscription key (channel:symbol[:suffix]) to SubscribeRequest
    /// IndexMap preserves insertion order for ordered reconnection
    subscriptions: RwLock<IndexMap<String, SubscribeRequest>>,

    /// Maps local subscription key to the server-assigned id from the
    /// `subscribed` event. Empty until the server acks; fallback path in
    /// unsubscribe uses the local key when a server id isn't recorded yet.
    server_ids: RwLock<HashMap<String, String>>,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(IndexMap::new()),
            server_ids: RwLock::new(HashMap::new()),
        }
    }

    /// Add a subscription to state
    ///
    /// From CONTEXT.md: "立即加入訂閱狀態" (immediately add to state)
    /// Subscriptions are stored even when disconnected, allowing restoration on reconnect.
    pub fn subscribe(&self, req: SubscribeRequest) {
        let key = req.key();
        let mut subs = self.subscriptions.write().unwrap();
        subs.insert(key, req);
    }

    /// Remove a subscription from state (also drops any recorded server id)
    ///
    /// From CONTEXT.md: "unsubscribe() 在斷線期間立即從狀態移除"
    /// Removes immediately even if disconnected.
    pub fn unsubscribe(&self, key: &str) {
        let mut subs = self.subscriptions.write().unwrap();
        subs.shift_remove(key);
        // Keep id map coherent — unsub drops any server id for this key. Use
        // a separate write() to avoid holding both locks simultaneously.
        drop(subs);
        self.server_ids.write().unwrap().remove(key);
    }

    /// Record the server-assigned subscription id for a local key.
    ///
    /// Called when a `subscribed` event arrives from the server. Overwrites
    /// any previous id for the same key (which is correct behavior: a fresh
    /// server id replaces the old one, e.g. on reconnect).
    pub fn record_server_id(&self, key: String, server_id: String) {
        self.server_ids.write().unwrap().insert(key, server_id);
    }

    /// Remove and return the recorded server id for a key.
    ///
    /// Returns `None` if the ack hasn't arrived yet (rare race on fast
    /// subscribe+unsubscribe), in which case the caller should fall back to
    /// sending the local key as the id so the wire format stays valid.
    pub fn take_server_id(&self, key: &str) -> Option<String> {
        self.server_ids.write().unwrap().remove(key)
    }

    /// Clear the server id map.
    ///
    /// Called on reconnect — every server id is now stale because the server
    /// will issue fresh ids on the new connection.
    pub fn clear_server_ids(&self) {
        self.server_ids.write().unwrap().clear();
    }

    /// Remove subscription by channel and symbol
    ///
    /// Convenience method that constructs the key.
    pub fn unsubscribe_by_channel_symbol(&self, channel: &str, symbol: &str) {
        let key = format!("{}:{}", channel, symbol);
        self.unsubscribe(&key);
    }

    /// Get all subscriptions in insertion order
    ///
    /// Returns cloned subscriptions for reconnection.
    /// IndexMap preserves insertion order.
    pub fn get_all(&self) -> Vec<SubscribeRequest> {
        let subs = self.subscriptions.read().unwrap();
        subs.values().cloned().collect()
    }

    /// Check if subscription exists
    pub fn contains(&self, key: &str) -> bool {
        let subs = self.subscriptions.read().unwrap();
        subs.contains_key(key)
    }

    /// Get number of active subscriptions
    pub fn count(&self) -> usize {
        let subs = self.subscriptions.read().unwrap();
        subs.len()
    }

    /// Clear all subscriptions (and server id map)
    pub fn clear(&self) {
        let mut subs = self.subscriptions.write().unwrap();
        subs.clear();
        drop(subs);
        self.server_ids.write().unwrap().clear();
    }

    /// Get all subscription keys
    pub fn keys(&self) -> Vec<String> {
        let subs = self.subscriptions.read().unwrap();
        subs.keys().cloned().collect()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Channel;

    #[test]
    fn test_subscribe_adds_to_state() {
        let manager = SubscriptionManager::new();
        let req = SubscribeRequest::new(Channel::Trades, "2330");

        manager.subscribe(req.clone());

        assert_eq!(manager.count(), 1);
        assert!(manager.contains("trades:2330"));

        let all = manager.get_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], req);
    }

    #[test]
    fn test_unsubscribe_removes_from_state() {
        let manager = SubscriptionManager::new();
        let req = SubscribeRequest::new(Channel::Trades, "2330");

        manager.subscribe(req.clone());
        assert_eq!(manager.count(), 1);

        manager.unsubscribe("trades:2330");
        assert_eq!(manager.count(), 0);
        assert!(!manager.contains("trades:2330"));
    }

    #[test]
    fn test_insertion_order_preserved() {
        let manager = SubscriptionManager::new();

        // Subscribe in specific order
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        manager.subscribe(SubscribeRequest::new(Channel::Candles, "2317"));
        manager.subscribe(SubscribeRequest::new(Channel::Books, "2454"));

        // get_all should return in insertion order
        let all = manager.get_all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].key(), "trades:2330");
        assert_eq!(all[1].key(), "candles:2317");
        assert_eq!(all[2].key(), "books:2454");
    }

    #[test]
    fn test_unsubscribe_during_disconnect_removes() {
        let manager = SubscriptionManager::new();

        // Simulate subscriptions during connection
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        manager.subscribe(SubscribeRequest::new(Channel::Candles, "2317"));
        assert_eq!(manager.count(), 2);

        // Simulate disconnect (state remains)
        // User calls unsubscribe during disconnection
        manager.unsubscribe("trades:2330");

        // Subscription should be removed from state
        assert_eq!(manager.count(), 1);
        assert!(!manager.contains("trades:2330"));
        assert!(manager.contains("candles:2317"));

        // get_all should only return remaining subscription
        let all = manager.get_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].key(), "candles:2317");
    }

    #[test]
    fn test_get_all_returns_in_order() {
        let manager = SubscriptionManager::new();

        // Add multiple subscriptions
        manager.subscribe(SubscribeRequest::new(Channel::Aggregates, "2330"));
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2317"));
        manager.subscribe(SubscribeRequest::new(Channel::Books, "2454"));
        manager.subscribe(SubscribeRequest::new(Channel::Candles, "2886"));

        let all = manager.get_all();
        assert_eq!(all.len(), 4);

        // Verify exact order matches insertion
        assert_eq!(all[0].key(), "aggregates:2330");
        assert_eq!(all[1].key(), "trades:2317");
        assert_eq!(all[2].key(), "books:2454");
        assert_eq!(all[3].key(), "candles:2886");
    }

    #[test]
    fn test_unsubscribe_by_channel_symbol() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));

        assert!(manager.contains("trades:2330"));

        manager.unsubscribe_by_channel_symbol("trades", "2330");

        assert!(!manager.contains("trades:2330"));
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_clear_removes_all() {
        let manager = SubscriptionManager::new();

        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        manager.subscribe(SubscribeRequest::new(Channel::Candles, "2317"));
        manager.subscribe(SubscribeRequest::new(Channel::Books, "2454"));

        assert_eq!(manager.count(), 3);

        manager.clear();

        assert_eq!(manager.count(), 0);
        assert!(manager.get_all().is_empty());
    }

    #[test]
    fn test_subscribe_updates_existing() {
        let manager = SubscriptionManager::new();

        let req1 = SubscribeRequest::new(Channel::Trades, "2330");
        manager.subscribe(req1);
        assert_eq!(manager.count(), 1);

        // Subscribe again with same key
        let req2 = SubscribeRequest::new(Channel::Trades, "2330");
        manager.subscribe(req2);

        // Count should still be 1 (update, not duplicate)
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_server_id_record_and_take() {
        let manager = SubscriptionManager::new();

        assert!(manager.take_server_id("trades:2330").is_none());

        manager.record_server_id("trades:2330".into(), "sub-xyz".into());
        assert_eq!(manager.take_server_id("trades:2330"), Some("sub-xyz".into()));

        // take consumes, second call is None
        assert!(manager.take_server_id("trades:2330").is_none());
    }

    #[test]
    fn test_server_id_overwrites_on_reconnect() {
        let manager = SubscriptionManager::new();

        manager.record_server_id("trades:2330".into(), "sub-old".into());
        // Reconnect scenario: server issues a fresh id for the same local key.
        manager.record_server_id("trades:2330".into(), "sub-new".into());

        assert_eq!(manager.take_server_id("trades:2330"), Some("sub-new".into()));
    }

    #[test]
    fn test_unsubscribe_drops_server_id() {
        let manager = SubscriptionManager::new();

        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        manager.record_server_id("trades:2330".into(), "sub-xyz".into());

        manager.unsubscribe("trades:2330");

        // After unsubscribe the id is gone, so a stale unsub wouldn't pick it up.
        assert!(manager.take_server_id("trades:2330").is_none());
    }

    #[test]
    fn test_clear_server_ids() {
        let manager = SubscriptionManager::new();

        manager.record_server_id("trades:2330".into(), "sub-a".into());
        manager.record_server_id("books:2317".into(), "sub-b".into());

        manager.clear_server_ids();

        assert!(manager.take_server_id("trades:2330").is_none());
        assert!(manager.take_server_id("books:2317").is_none());
    }

    #[test]
    fn test_clear_also_clears_server_ids() {
        let manager = SubscriptionManager::new();

        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        manager.record_server_id("trades:2330".into(), "sub-xyz".into());

        manager.clear();

        assert_eq!(manager.count(), 0);
        assert!(manager.take_server_id("trades:2330").is_none());
    }
}
