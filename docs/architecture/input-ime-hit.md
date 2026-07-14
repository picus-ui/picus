# Input, IME, and hit testing

Input is window-scoped. Pointer coordinates are read from the event window's
physical cursor position and passed to the matching `WindowRuntime`; they are
not read from whichever window happens to be primary. The retained runtime
converts physical coordinates to logical coordinates when it performs hit
testing.

The input contract is deliberately ordered:

1. Window input events update the runtime cursor and interaction state.
2. Click injection sends a move before the down/up pair so hover and hit paths
   are current when the click is dispatched.
3. `RetainedRouting` routes the event through the deepest hit target.
4. `DispatchActions` converts retained actions into `UiAction<T>` messages.

Resize events use logical dimensions. This keeps layout, projection, and the
retained view in the same coordinate space even when a window has a scale
factor. The event's window entity remains the source of truth for both resize
and pointer routing.

Text input and IME composition are synchronized during `PostUpdate` after
projection changes. Focus and composition state stay with the window runtime;
applications consume completed business actions through `MessageReader`, not by
reading retained input queues. Clipboard, drag/drop, accelerator, and
accessibility requests follow the same per-window routing boundary.

For overlay behavior, outside-click dismissal examines the top overlay hit path
and its bound widget IDs. Nested wheel routing starts at the deepest hit target;
see [overlays and scroll](../guide/overlays-scroll.md).
