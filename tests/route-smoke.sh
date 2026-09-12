#!/usr/bin/env bash
set -euo pipefail

base_url="${BASE_URL:-http://localhost:8787}"
status() {
  /usr/bin/curl -sS -o /dev/null -w '%{http_code}' "$base_url$1"
}

for path in / /blog /rss.xml /robots.txt /sitemap.xml; do
  test "$(status "$path")" = 200 || { echo "expected 200: $path" >&2; exit 1; }
done

test "$(status /missing)" = 404 || { echo "expected 404: /missing" >&2; exit 1; }
test "$(/usr/bin/curl -sS "$base_url/missing" | /usr/bin/grep -c 'Page not found')" -gt 0 || {
  echo "expected branded 404 body" >&2
  exit 1
}

test -f dist/_headers || { echo "expected generated root _headers" >&2; exit 1; }
test ! -f dist/assets/_headers || { echo "stale nested _headers must not be deployed" >&2; exit 1; }
/usr/bin/grep -Fq '/assets/*' dist/_headers || { echo "expected /assets/* header rule" >&2; exit 1; }

test "$(/usr/bin/curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' "$base_url/blog/coding-confortable")" = "308 $base_url/blog/coding-comfortable" || {
  echo "legacy redirect failed" >&2
  exit 1
}

while IFS= read -r post; do
  slug="${post##*/}"
  slug="${slug%.md}"
  test "$(status "/blog/$slug")" = 200 || { echo "post route failed: $slug" >&2; exit 1; }
done < <(find content/posts -name '*.md' -print | sort)

echo "route smoke passed"
