 == ".pdf"
    assert out.read_bytes() == b"%PDF-1.7 test"


@pytest.mark.parametrize(
    ("url", "expected"),
    [
        ("https://x.com/user/status/1", "https://twitter.com/user/status/1"),
        ("https://mobile.x.com/user/status/1", "https://mobile.twitter.com/user/status/1"),
        ("https://X.COM/user/status/1", "https://twitter.com/user/status/1"),
        # "x.com" outside the host must survive untouched.
        (
            "https://x.com/user/status/1?next=https://x.com/home",
            "https://twitter.com/user/status/1?next=https://x.com/home",
        ),
        (
            "https://twitter.com/dropbox.com_fan/status/1",
            "https://twitter.com/dropbox.com_fan/status/1",
        ),
    ],
)
def test_tweet_oembed_rewrites_only_the_host(url, expected, monkeypatch):
    """The x.com -> twitter.com rewrite for oEmbed was a replace over the whole
    URL, so "x.com" inside the path or query (e.g. a dropbox.com handle, a
    ?next= link) was rewritten too, sending oEmbed a different URL."""
    requested = []

    def fake_fetch_text(u):
        requested.append(u)
        return '{"html": "<p>hi</p>", "author_name": "someone"}'

    monkeypatch.setattr(ingest_mod, "safe_fetch_text", fake_fetch_text)

    ingest_mod._fetch_tweet(url, None, None)

    query = urllib.parse.parse_qs(urllib.parse.urlsplit(requested[0]).query)
    assert query["url"] == [expected]
