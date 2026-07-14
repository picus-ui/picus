# Markdown streaming

The markdown projection is designed for append-heavy content such as a chat or
code stream. Keep a cache for the parsed prefix and the rendered blocks. When a
new chunk extends the existing text, reuse the unchanged prefix and parse only
the suffix that can have changed. A replacement or edit before the cached
boundary invalidates the affected suffix and rebuilds from that point.

Streaming updates should be monotonic from the application perspective:

- append incoming text to the source buffer;
- retain parsed blocks whose source range is unchanged;
- update the final incomplete block while the stream is active;
- mark the cache finished once the producer closes the stream.

The finalization step matters for unfinished fences, emphasis, and paragraph
boundaries. A completed stream must produce the same blocks as parsing the full
document once. Do not wholesale rebuild the parse cache on every append when the
prefix is unchanged.

Tests should cover empty input, one-chunk input, multiple appends, edits before
the cache boundary, unfinished markdown followed by a final chunk, and a full
parse versus incremental parse comparison. The `picuscode` example exercises
the application-side streaming path; its CodeWhale integration tests must use
fixtures and never touch the user's real `~/.codewhale/` directory.
