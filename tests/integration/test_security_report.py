import json
import pytest

CSP_IGNORED_FIELDS = (
    "event_id",
    "timestamp",
    "received",
    "ingest_path",
)
EXPECT_CT_IGNORED_FIELDS = ("event_id", "ingest_path")
EXPECT_STAPLE_IGNORED_FIELDS = ("event_id", "ingest_path")
HPKP_IGNORED_FIELDS = ("event_id", "ingest_path")


def get_security_report(envelope):
    if envelope is not None:
        for item in envelope.items:
            if item.headers.get("type") == "security":
                return json.loads(item.get_bytes())


def id_fun1(origins):
    allowed, should_be_allowed = origins
    should_it = "" if should_be_allowed else "not"
    return f"{str(allowed)} should {should_it} be allowed "


@pytest.mark.parametrize(
    "allowed_origins",
    [(["valid.com"], True), (["invalid.com"], False)],
    ids=id_fun1,
)
def test_uses_origins(mini_sentry, relay, json_fixture_provider, allowed_origins):
    allowed_domains, should_be_allowed = allowed_origins
    fixture_provider = json_fixture_provider(__file__)
    proj_id = 42
    relay = relay(mini_sentry)
    report = fixture_provider.load("csp", ".input")
    mini_sentry.add_full_project_config(proj_id)

    relay.send_security_report(
        project_id=proj_id,
        content_type="application/json; charset=utf-8",
        payload=report,
        release="01d5c3165d9fbc5c8bdcf9550a1d6793a80fc02b",
        environment="production",
        origin="http://valid.com",
    )

    if should_be_allowed:
        mini_sentry.captured_events.get(timeout=10).get_event()
    assert mini_sentry.captured_events.empty()


@pytest.mark.parametrize(
    "test_case",
    [
        ("csp", CSP_IGNORED_FIELDS),
    ],
    ids=("csp",),
)
def test_security_report_with_processing(
    mini_sentry,
    relay_with_processing,
    events_consumer,
    test_case,
    json_fixture_provider,
):
    # UA parsing introduces higher latency in debug mode
    events_consumer = events_consumer(timeout=10)

    fixture_provider = json_fixture_provider(__file__)
    test_name, ignored_properties = test_case
    proj_id = 42
    relay = relay_with_processing()
    report = fixture_provider.load(test_name, ".input")
    mini_sentry.add_full_project_config(proj_id)

    relay.send_security_report(
        project_id=proj_id,
        content_type="application/json; charset=utf-8",
        payload=report,
        release="01d5c3165d9fbc5c8bdcf9550a1d6793a80fc02b",
        environment="production",
    )

    event, _ = events_consumer.get_event()
    for x in ignored_properties:
        event.pop(x, None)

    ext = ".normalized.output"
    expected_evt = fixture_provider.load(test_name, ext)

    event.pop("_metrics", None)
    assert event == expected_evt


@pytest.mark.parametrize(
    "test_case",
    [
        ("csp", CSP_IGNORED_FIELDS),
        ("csp_chrome", CSP_IGNORED_FIELDS),
        ("csp_chrome_blocked_asset", CSP_IGNORED_FIELDS),
        ("csp_firefox_blocked_asset", CSP_IGNORED_FIELDS),
        ("expect_ct", EXPECT_CT_IGNORED_FIELDS),
        ("expect_staple", EXPECT_STAPLE_IGNORED_FIELDS),
        ("hpkp", HPKP_IGNORED_FIELDS),
    ],
    ids=(
        "csp",
        "csp_chrome",
        "csp_chrome_blocked_asset",
        "csp_firefox_blocked_asset",
        "expect_ct",
        "expect_staple",
        "hpkp",
    ),
)
def test_security_report(mini_sentry, relay, test_case, json_fixture_provider):
    fixture_provider = json_fixture_provider(__file__)
    test_name, ignored_properties = test_case
    proj_id = 42
    relay = relay(mini_sentry)
    report = fixture_provider.load(test_name, ".input")
    mini_sentry.add_full_project_config(proj_id)

    resp = relay.send_security_report(
        project_id=proj_id,
        content_type="application/json",
        payload=report,
        release="01d5c3165d9fbc5c8bdcf9550a1d6793a80fc02b",
        environment="production",
    )

    assert resp.status_code == 200

    envelope = mini_sentry.captured_events.get(timeout=10)
    event = get_security_report(envelope)
    for x in ignored_properties:
        event.pop(x, None)

    ext = ".no_processing.output"
    expected_evt = fixture_provider.load(test_name, ext)

    if "received" in event:
        event.pop("received")
    if "timestamp" in event:
        event.pop("timestamp")

    assert event == expected_evt


def split_header(header_val):
    return [x.strip() for x in header_val.split(",")]


def test_security_report_preflight(mini_sentry, relay):
    """
    Test that we respond correctly to a CORS preflight request
    """
    proj_id = 42
    relay = relay(mini_sentry)
    project_config = mini_sentry.add_full_project_config(proj_id)
    dsn = project_config["publicKeys"][0]
    url = "/api/{}/security/?sentry_key={}".format(proj_id, dsn)
    headers = {
        "Host": "relay.example.org",
        "Pragma": "no-cache",
        "Cache-Control": "no-cache",
        "Origin": None,
        "Access-Control-Request-Method": "POST",
        "Access-Control-Request-Headers": "content-type",
        "User-Agent": "Some Browser",
        "Accept-Encoding": "gzip, deflate, br",
        "Accept-Language": "en-GB,en;q=0.9,en-US;q=0.8",
        "X-Forwarded-Proto": "https",
        "X-Forwarded-For": "2a02:8388:8b86:a700:541e:f608:70f6:bcf2",
    }
    resp = relay.req_options(url, headers=headers)
    assert resp.status_code == 200
    headers = resp.headers

    assert headers["access-control-allow-methods"] == "POST"
    assert set(split_header(headers["access-control-allow-headers"])) == set(
        [
            "x-forwarded-for",
            "content-type",
            "transfer-encoding",
            "referer",
            "authorization",
            "origin",
            "authentication",
            "content-encoding",
            "x-sentry-auth",
            "accept",
            "x-requested-with",
        ]
    )


def test_security_report_expose_headers(mini_sentry, relay):
    relay = relay(mini_sentry)
    mini_sentry.add_full_project_config(42)

    resp = relay.send_security_report(
        project_id=42,
        content_type="application/json; charset=utf-8",
        payload="",  # Invalid, but irrelevant for this test
        release="01d5c3165d9fbc5c8bdcf9550a1d6793a80fc02b",
        environment="production",
        origin="http://valid.com",
    )

    assert set(split_header(resp.headers["access-control-expose-headers"])) == set(
        ["x-sentry-error", "x-sentry-rate-limits", "retry-after"]
    )


def test_adds_origin_header(mini_sentry, relay, json_fixture_provider):
    fixture_provider = json_fixture_provider(__file__)
    proj_id = 42
    relay = relay(mini_sentry)
    report = fixture_provider.load("csp", ".input")
    mini_sentry.add_full_project_config(proj_id)

    relay.send_security_report(
        project_id=proj_id,
        content_type="application/json; charset=utf-8",
        payload=report,
        release="01d5c3165d9fbc5c8bdcf9550a1d6793a80fc02b",
        environment="production",
        origin="http://valid.com",
    )

    envelope = mini_sentry.captured_events.get(timeout=10)
    event = get_security_report(envelope)

    assert ["Origin", "http://valid.com/"] in event["request"]["headers"]
