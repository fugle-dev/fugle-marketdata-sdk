// credentials.go - Replacing a client's credential (#322)
package marketdata_uniffi

// credentialsFrom reads the credential options WithApiKey, WithBearerToken
// and WithSdkToken into the record core checks. Any other option is a
// ConfigError (code 1004): it would have no effect on a client already built.
//
// Other options are found by what they leave in the config, so one that
// leaves its zero value, such as WithBaseUrl(""), goes unnoticed; it would
// have changed nothing either.
func credentialsFrom(opts []Option) (CredentialsRecord, error) {
	cfg := &clientConfig{}
	for _, opt := range opts {
		if err := opt(cfg); err != nil {
			return CredentialsRecord{}, err
		}
	}
	record := CredentialsRecord{ApiKey: &cfg.apiKey, BearerToken: &cfg.bearerToken, SdkToken: &cfg.sdkToken}
	rest := *cfg
	rest.apiKey, rest.bearerToken, rest.sdkToken = "", "", ""
	if rest != (clientConfig{}) {
		const msg = "SetCredentialsWith takes only WithApiKey, WithBearerToken and WithSdkToken"
		return CredentialsRecord{}, NewMarketDataErrorConfigError(msg, ErrorInfo{
			Code:       1004,
			SourceKind: ErrorSourceKindClient,
			Message:    "Configuration error: " + msg,
		})
	}
	return record, nil
}

// SetCredentialsWith replaces the credential later requests send, like
// SetCredentials, taking the constructor's credential options:
//
//	err := client.SetCredentialsWith(marketdata_uniffi.WithSdkToken(newToken))
//
// Exactly one non-empty credential must be given; otherwise, or if it cannot
// be sent in an HTTP header, it returns a ConfigError (code 1004) and the
// current credential is kept. Clients already taken from this one (Stock(),
// Stock().Intraday(), ...) send it too.
func (_self *RestClient) SetCredentialsWith(opts ...Option) error {
	record, err := credentialsFrom(opts)
	if err != nil {
		return err
	}
	return _self.SetCredentials(record)
}

// SetCredentials replaces the credential this client authenticates with from
// its next connection attempt on: the next Connect or automatic reconnect.
// Any of the three kinds may replace any other. A connection already
// authenticated is not authenticated again, so call it before a token
// expires. A first Connect whose credential is rejected leaves the client
// usable: set a new credential and Connect again. A rejection during an
// automatic reconnect ends it, and with it Messages(): create a new client
// with the new credential to stream again.
//
// Exactly one non-empty credential must be set; otherwise it returns a
// ConfigError (code 1004) and the current credential is kept. After Close()
// it returns ClientClosed (code 2010).
func (sc *StreamingClient) SetCredentials(credentials CredentialsRecord) error {
	// Held so Close() cannot destroy the client during the call.
	sc.mu.RLock()
	defer sc.mu.RUnlock()
	if sc.closed {
		return NewMarketDataErrorClientClosed(ErrorInfo{
			Code:       2010,
			SourceKind: ErrorSourceKindClient,
			Message:    "Client closed: create a new client to set credentials",
		})
	}
	return sc.client.SetCredentials(credentials)
}

// SetCredentialsWith is SetCredentials taking the constructor's credential
// options:
//
//	err := client.SetCredentialsWith(marketdata_uniffi.WithSdkToken(newToken))
func (sc *StreamingClient) SetCredentialsWith(opts ...Option) error {
	record, err := credentialsFrom(opts)
	if err != nil {
		return err
	}
	return sc.SetCredentials(record)
}
