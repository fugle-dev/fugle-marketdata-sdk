#!/usr/bin/env python3
"""Exchange a Fubon brokerage login for a Fugle realtime SDK token.

The Rust sweep (`core/examples/prod_smoke.rs`) authenticates with either a
Fugle developer API key or a broker-issued realtime token. Some endpoints —
`stock/ownership/etf-holdings` in particular — are only granted to real
brokerage accounts, so the token path is the only way to exercise them.

This prints **only** the token to stdout, so it can be captured directly into
an environment variable without the secret landing in a file or in shell
history. Everything else goes to stderr.

Usage (run with the fubon_neo venv from your sdk-demo checkout):

    DEMO=~/Project/fubon/sdk-demo
    set -a; . "$DEMO/profiles/dev.env"; set +a
    export FUGLE_SDK_TOKEN=$("$DEMO/python/.venv-dev/bin/python" \\
        core/examples/fubon_token.py)

Reads FUBON_WS, FUBON_ID, FUBON_PWD, FUBON_CERT, FUBON_CERT_PWD from the
environment — the same variables the sdk-demo profiles define.

This performs a real brokerage login. It only exchanges a market-data token;
it places no orders.
"""

import os
import sys


def main() -> int:
    try:
        from fubon_neo.sdk import FubonSDK, Mode
    except ImportError as exc:
        print(
            f"fubon_neo not importable ({exc}).\n"
            "Run this with the sdk-demo venv, e.g.\n"
            "  ~/Project/fubon/sdk-demo/python/.venv-dev/bin/python "
            "core/examples/fubon_token.py",
            file=sys.stderr,
        )
        return 2

    required = ["FUBON_WS", "FUBON_ID", "FUBON_PWD", "FUBON_CERT"]
    missing = [name for name in required if not os.environ.get(name)]
    if missing:
        print(
            f"missing environment: {', '.join(missing)}. "
            "Source a profile first, e.g. `set -a; . profiles/dev.env; set +a`.",
            file=sys.stderr,
        )
        return 2

    # Deliberately not defaulted: the prod profile sets this to an empty string
    # on purpose (prod certificates carry no password), and guessing would turn
    # a config mistake into a confusing auth failure.
    cert_pwd = os.environ.get("FUBON_CERT_PWD", "")

    sdk = FubonSDK(30, 2, os.environ["FUBON_WS"])
    result = sdk.login(
        os.environ["FUBON_ID"],
        os.environ["FUBON_PWD"],
        os.environ["FUBON_CERT"],
        cert_pwd,
    )
    if not getattr(result, "is_success", False):
        print(f"login failed: {getattr(result, 'message', result)}", file=sys.stderr)
        return 1
    print(f"login ok: {os.environ['FUBON_ID']} @ {os.environ['FUBON_WS']}", file=sys.stderr)

    # `init_realtime` is what performs the token exchange; the token it used is
    # not exposed as a property, so call the underlying exchange directly.
    sdk.init_realtime(Mode.Speed)
    token = sdk.exchange_realtime_token()
    if not token:
        print("exchange_realtime_token() returned nothing", file=sys.stderr)
        return 1

    print(f"token acquired ({len(token)} chars)", file=sys.stderr)
    print(token)  # stdout: the token and nothing else
    return 0


if __name__ == "__main__":
    sys.exit(main())
