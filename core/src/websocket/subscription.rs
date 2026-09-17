//! WebSocket subscription state management with ordered preservation

use crate::models::SubscribeRequest;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
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

    /// Server ids and cancels awaiting an ack, under one lock so an
    /// unsubscribe and the ack it waits for cannot miss each other (#136).
    ids: RwLock<ServerIds>,
}

#[derive(Default)]
struct ServerIds {
    /// Local subscription key to the server-assigned id from the
    /// `subscribed` event. Empty until the server acks.
    by_key: HashMap<String, String>,
    /// Keys unsubscribed before their ack arrived. The server only accepts
    /// the id it issued, so the unsubscribe is sent when the ack brings it.
    ///
    /// An entry is dropped by the ack it waits for, by subscribing the key
    /// again, by the next [`SubscriptionManager::clear_server_ids`] (the
    /// connection it was waiting on is gone), or by
    /// [`SubscriptionManager::clear`].
    pending_cancels: HashSet<String>,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(IndexMap::new()),
            ids: RwLock::new(ServerIds::default()),
        }
    }

    /// Add a subscription to state
    ///
    /// From CONTEXT.md: "立即加入訂閱狀態" (immediately add to state)
    /// Subscriptions are stored even when disconnected, allowing restoration on reconnect.
    /// Subscribing again drops a cancel still waiting for this key's ack.
    pub fn subscribe(&self, req: SubscribeRequest) {
        let key = req.key();
        // Lock order: `ids`, then `subscriptions`, and both in one critical
        // section so a `subscribed` ack in flight cannot see the cancel this
        // call drops and unsubscribe the subscription it just added (#136).
        let mut ids = self.ids.write().unwrap();
        self.subscriptions.write().unwrap().insert(key.clone(), req);
        ids.pending_cancels.remove(&key);
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
        self.ids.write().unwrap().by_key.remove(key);
    }

    /// Record the server-assigned subscription id for a local key.
    ///
    /// The ack path uses [`Self::record_ack`] instead, which also answers a
    /// cancel that arrived before the ack; this one only records.
    ///
    /// Overwrites any previous id for the same key (which is correct
    /// behavior: a fresh server id replaces the old one, e.g. on reconnect).
    pub fn record_server_id(&self, key: String, server_id: String) {
        self.ids.write().unwrap().by_key.insert(key, server_id);
    }

    /// Record the id a `subscribed` ack carries for `key`. Returns the id
    /// instead when `key` was unsubscribed before the ack: the caller sends
    /// the unsubscribe for it, and the id is not kept.
    pub(crate) fn record_ack(&self, key: String, server_id: String) -> Option<String> {
        let mut ids = self.ids.write().unwrap();
        if ids.pending_cancels.remove(&key) {
            return Some(server_id);
        }
        ids.by_key.insert(key, server_id);
        None
    }

    /// Remove and return the recorded server id for a key.
    ///
    /// Returns `None` if the ack hasn't arrived yet. Unsubscribing uses
    /// [`Self::resolve_unsubscribe`] instead, which also accepts a server id
    /// and records a cancel when the ack is still outstanding.
    pub fn take_server_id(&self, key: &str) -> Option<String> {
        self.ids.write().unwrap().by_key.remove(key)
    }

    /// Remove what `target` names from state and return the id to send in
    /// the unsubscribe frame, if any. `target` is either:
    ///
    /// - a local key with a recorded server id: sends that id;
    /// - a recorded server id: removes every key the id was issued for
    ///   (a FutOpt alias and its contract share one) and sends it;
    /// - a local key whose ack has not arrived: nothing is sent now, the
    ///   unsubscribe goes out when the ack does (see [`Self::record_ack`]);
    /// - anything else: sent as is, as the server may know the id.
    pub(crate) fn resolve_unsubscribe(&self, target: &str) -> Option<String> {
        // Lock order: `ids`, then `subscriptions`.
        let mut ids = self.ids.write().unwrap();
        if let Some(id) = ids.by_key.remove(target) {
            self.subscriptions.write().unwrap().shift_remove(target);
            return Some(id);
        }
        let keys: Vec<String> = ids
            .by_key
            .iter()
            .filter(|(_, id)| id.as_str() == target)
            .map(|(key, _)| key.clone())
            .collect();
        if !keys.is_empty() {
            let mut subs = self.subscriptions.write().unwrap();
            for key in &keys {
                ids.by_key.remove(key);
                subs.shift_remove(key);
            }
            return Some(target.to_string());
        }
        if self.subscriptions.write().unwrap().shift_remove(target).is_some() {
            ids.pending_cancels.insert(target.to_string());
            return None;
        }
        Some(target.to_string())
    }

    /// Clear the server id map.
    ///
    /// Called on reconnect, before the subscriptions to replay are read:
    /// every server id is now stale because the server will issue fresh ids
    /// on the new connection.
    ///
    /// Cancels awaiting an ack are dropped along with them unless the key is
    /// still subscribed: a key this call leaves unsubscribed is not replayed,
    /// so no ack for it can arrive and the cancel would sit there forever. A
    /// cancel recorded after this call — the unsubscribe that races a replay
    /// already under way — is kept, and the new connection's ack answers it.
    pub fn clear_server_ids(&self) {
        // Lock order: `ids`, then `subscriptions`.
        let mut ids = self.ids.write().unwrap();
        ids.by_key.clear();
        let subs = self.subscriptions.read().unwrap();
        ids.pending_cancels.retain(|key| subs.contains_key(key));
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
        *self.ids.write().unwrap() = ServerIds::default();
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
    #[test]
    fn resolve_local_key_sends_recorded_id() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        assert_eq!(manager.record_ack("trades:2330".into(), "sub-a".into()), None);

        assert_eq!(manager.resolve_unsubscribe("trades:2330"), Some("sub-a".into()));
        assert_eq!(manager.count(), 0);
        assert!(manager.take_server_id("trades:2330").is_none());
    }

    #[test]
    fn resolve_server_id_removes_local_subscription() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        manager.subscribe(SubscribeRequest::new(Channel::Books, "2317"));
        manager.record_ack("trades:2330".into(), "sub-a".into());
        manager.record_ack("books:2317".into(), "sub-b".into());

        assert_eq!(manager.resolve_unsubscribe("sub-a"), Some("sub-a".into()));
        assert_eq!(manager.keys(), ["books:2317"]);
        assert!(manager.take_server_id("trades:2330").is_none());
    }

    #[test]
    fn resolve_server_id_removes_every_key_it_was_issued_for() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "TXF1"));
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "TXFK6"));
        manager.record_ack("trades:TXF1".into(), "sub-a".into());
        manager.record_ack("trades:TXFK6".into(), "sub-a".into());

        assert_eq!(manager.resolve_unsubscribe("sub-a"), Some("sub-a".into()));
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn resolve_before_ack_sends_id_when_ack_arrives() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));

        assert_eq!(manager.resolve_unsubscribe("trades:2330"), None);
        assert_eq!(manager.count(), 0);

        assert_eq!(
            manager.record_ack("trades:2330".into(), "sub-a".into()),
            Some("sub-a".into())
        );
        assert!(manager.take_server_id("trades:2330").is_none());
        // Answered once: a later ack for the key is recorded again.
        assert_eq!(manager.record_ack("trades:2330".into(), "sub-b".into()), None);
    }

    #[test]
    fn subscribing_again_drops_pending_cancel() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        assert_eq!(manager.resolve_unsubscribe("trades:2330"), None);

        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));

        assert_eq!(manager.record_ack("trades:2330".into(), "sub-a".into()), None);
        assert!(manager.contains("trades:2330"));
    }

    #[test]
    fn clear_server_ids_drops_a_cancel_whose_key_is_gone() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));
        assert_eq!(manager.resolve_unsubscribe("trades:2330"), None);

        // The key is not subscribed any more, so the replay leaves it out and
        // no ack for it can arrive: the cancel goes with the stale ids.
        manager.clear_server_ids();

        assert_eq!(manager.record_ack("trades:2330".into(), "sub-a".into()), None);
    }

    #[test]
    fn a_cancel_recorded_after_clear_server_ids_is_kept() {
        let manager = SubscriptionManager::new();
        manager.subscribe(SubscribeRequest::new(Channel::Trades, "2330"));

        // Reconnect: ids cleared, then the replay goes out. An unsubscribe
        // landing here is answered by the new connection's ack.
        manager.clear_server_ids();
        assert_eq!(manager.resolve_unsubscribe("trades:2330"), None);

        assert_eq!(
            manager.record_ack("trades:2330".into(), "sub-a".into()),
            Some("sub-a".into())
        );
    }

    #[test]
    fn resolve_unknown_target_is_sent_as_is() {
        let manager = SubscriptionManager::new();
        assert_eq!(manager.resolve_unsubscribe("sub-x"), Some("sub-x".into()));
    }
}
