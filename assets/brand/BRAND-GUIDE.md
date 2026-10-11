# DNACode Icon & Wordmark — v1 proposal

## Review verdict
**Refine the current DNA concept rather than replace it.** The DNA metaphor and dark rounded tile already support the name DNACode. The revised construction uses a solid secondary blue instead of opacity, a more balanced strand/rung ratio, optical rung joins aligned with the strand centerlines, and a unified source for the desktop icon and brand lockup.

The 16px version remains an intentionally compact app glyph; no 16px icon can preserve all DNA detail. The deliverable therefore uses a pixel-size-tuned version at 16/24px and a separate 32px vector before returning to the full master from 48px upward.

## Assets
- `DNACode-icon-master.svg` — editable 1024×1024 vector master, dark app icon.
- `DNACode-icon-1024.png` — 1024×1024 PNG generated from the vector master.
- `DNACode-icon-small.svg` — optically simplified 24×24 source for 16px and 24px.
- `DNACode-icon-16.png`, `DNACode-icon-24.png` — small-size exports from the compact source.
- `DNACode-icon-32.svg`, `DNACode-icon-32.png` — 32px-specific optical version.
- `DNACode-icon-48.png` through `DNACode-icon-512.png` — exports from the full master.
- `DNACode-icon-light.svg` / `.png` — light-background variant.
- `DNACode-icon-monochrome.svg` / `.png` — one-colour variant.
- `DNACode-wordmark.svg` — horizontal icon + wordmark lockup.
- `preview-sizes.png` — dimensions and variant preview.
- `current-vs-refined.png` — comparison with the provided current icon.

## Colour
- Brand blue: `#4C8DFF`
- Rear strand: `#315FA8` (opaque; avoids alpha-induced muddy pixels)
- Dark tile: `#151B26` to `#1B2230`
- Light variant blue: `#2563EB`
- Light wordmark text: `#E6E9F0`

## Size strategy
- **16px:** use the dedicated `DNACode-icon-16.png`. It has a thicker stroke, compact geometry, opaque background, and slightly brighter rungs drawn over both strands to keep the ladder structure visible after rasterization.
- **24px:** use `DNACode-icon-24.png` / `DNACode-icon-small.svg`. The rungs intentionally overlay both strands at this size to prioritise the ladder read over a literal weave.
- **32px:** use `DNACode-icon-32.png` / `DNACode-icon-32.svg`, tuned to a moderate pixel stroke and rungs drawn over both strands rather than simply scaling the 24px source.
- **48px and above:** use the full 1024px master or supplied size exports.
- For multi-resolution ICO/ICNS assets, include the art-directed small bitmaps if the packaging pipeline supports it. A one-source conversion of the 1024px master may discard the small-size optical tuning; verify packaged 16px icons in Windows Explorer/taskbar and the target OS launcher.

## Construction principles
- Keep the dark rounded-square tile and DNA as the central metaphor.
- Use the same geometry for the main icon and wordmark icon.
- Draw the rear strand first, then rungs, then the front strand to keep the over-under relationship consistent.
- Use a solid darker blue for the rear strand rather than 59% alpha.
- Rung centerlines meet the strand centerlines; the overlap caused by rounded strokes is deliberate optical compensation.
- Do not place the wordmark inside the app-icon square.

## Wordmark use
- Use `DNACode-wordmark.svg` in README banners, website headers, and documentation.
- Use the square icon alone for OS application icons and compact in-app placements.
- Leave at least one foreground-stroke width of clear space around the lockup.
- For dark surfaces, use the wordmark with the dark icon. For light surfaces, use the light icon and dark type as needed; avoid arbitrary recolouring.

## Build integration
The PNG files are standalone assets. The actual Windows `.ico`, macOS `.icns` and Linux icon generation pipeline should be updated deliberately to include art-directed sizes. Do not overwrite the small-size variants by feeding only the 1024px PNG to a one-source converter without checking 16px output.

## Licence
Intended for use and distribution with the MIT-licensed DNACode project. Include the SVG source files with the project so contributors can edit them.
