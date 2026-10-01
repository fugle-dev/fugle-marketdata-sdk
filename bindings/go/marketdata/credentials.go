// credentials.go - Replacing a client's credential (#322)
package marketdata_uniffi

import "errors"

// credentialsFrom reads the credential options WithApiKey, WithBearerToken
// and WithSdkToken into the record core checks. Any other option is an
// error: it would have no effect on a client already built.
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
		return CredentialsRecord{}, errors.New("SetCredentialsWith takes only WithApiKey, WithBearerToken and WithSdkToken")
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
// ConfigError (code 1004) and the current credential is kept.
func (sc *StreamingClient) SetCredentials(credentials CredentialsRecord) error {
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
