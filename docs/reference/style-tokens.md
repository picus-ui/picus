# Style tokens and themes

Stylesheets are RON data resolved by selector, class, and inline precedence.
Keep production colours, typography, spacing, and state colours in the
stylesheet or its token table; `picus_widget` remains lookless and does not
provide a production brand palette.

Token values commonly fall into these groups:

- color and brush values for text, fills, borders, and overlays;
- spacing and sizing values for padding, gaps, radii, and dimensions;
- typography values for font family, size, weight, and line height;
- interaction values for hover, pressed, disabled, focus, and transitions;
- window values such as the optional native backdrop material and color scheme.

No theme or no matching rule means no framework-provided visible default. A
partial stylesheet is valid: an absent component rule or token leaves that
property transparent or empty. Only structural RON errors, invalid value types,
and invalid token references fail loading. The framework never silently chooses
dark or light.

Applications can load RON with `load_style_sheet_ron`, load a stylesheet asset,
or select a variant with `style_variant`. Inline builders and classes are useful
for local overrides, but should not become a replacement for the application's
stylesheet contract. Test skins belong in `picus_theme_test`, not in the
lookless widget crate.
