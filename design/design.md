<!-- Hallmark · pre-emit critique: P5 H5 E5 S5 R5 V5 -->
<!-- Hallmark · studied: yes · DNA-source: url + user-attached images
     source-url: https://github.com/zqq-699/BlockHelm-Launcher
     source-revision: b0607fd3b4b36cba129d28af505a8b63592ad7a7 -->

# Design — BlockHelm Workbench

Locked design system. Future design and implementation work should read this
file first and defer to it. Amend intentionally; do not create per-page themes
that drift from the system.

## System

- Genre · modern-minimal desktop utility
- Macrostructure · Workbench, with Index-First traits on catalogue pages
- Theme · studied-DNA with paired `Light / Azure` and `Dark / Forest` variants
- Axes · light-or-dark paper / neutral system sans / swappable accent hue
- Character · quiet acrylic workspace, stable navigation skeleton, one vivid action
- Primary navigation · N3 Side-rail, left, 62 px collapsed and 176 px expanded
- Context navigation · detached 224 px panel with 24 px outer inset
- Content · fixed title/search layer over a restrained, medium-density work surface

## Provenance

- Source mode · public URL, supplemented by four user-attached screenshots for rhythm
- Source · <https://github.com/zqq-699/BlockHelm-Launcher>
- Source revision · `b0607fd3b4b36cba129d28af505a8b63592ad7a7`
- Extracted · 2026-08-13
- Reference licence · [GNU GPL v3.0](https://github.com/zqq-699/BlockHelm-Launcher/blob/b0607fd3b4b36cba129d28af505a8b63592ad7a7/LICENSE)
- Attestation · category (b): public GPL-3.0 reference for the user's own open-source software
- User intent · non-commercial distribution, with source published and GPL obligations followed
- Confidence · colour, font, geometry, and motion facts come from the referenced source revision; density and visual rhythm come from the screenshots
- Boundary · this document extracts structure and design logic, not Minecraft branding, copy, icons, artwork, or pixel assets
- Licence note · if BlockHelm code or assets are copied or adapted, preserve applicable notices and satisfy [GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html) requirements for the covered work and corresponding source. Independently reimplementing abstract visual principles may require a different analysis. This is provenance guidance, not legal advice.

## Tokens

Canonical values are expressed as semantic CSS custom properties. When a real
project is created, move this block into `tokens.css`; that file then becomes
the source of truth.

```css
:root {
  color-scheme: light dark;

  --font-display: "Microsoft YaHei UI", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
  --font-body: "Microsoft YaHei UI", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
  --font-mono: "Cascadia Mono", Consolas, ui-monospace, monospace;

  --space-3xs: 4px;
  --space-2xs: 8px;
  --space-xs: 12px;
  --space-sm: 16px;
  --space-md: 24px;
  --space-lg: 32px;
  --space-xl: 48px;
  --space-2xl: 64px;

  --text-xs: 11px;
  --text-sm: 12px;
  --text-base: 14px;
  --text-md: 16px;
  --text-lg: 20px;
  --text-title: 24px;

  --radius-caption: 6px;
  --radius-input: 8px;
  --radius-card: 10px;
  --radius-nav: 12px;
  --radius-panel: 14px;

  --rail-width-collapsed: 62px;
  --rail-width-expanded: 176px;
  --context-panel-width: 224px;
  --content-max-width: 800px;

  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-in-out: cubic-bezier(0.65, 0, 0.35, 1);
  --dur-fast: 160ms;
  --dur-exit: 220ms;
  --dur-base: 240ms;
  --dur-slow: 320ms;
  --dur-rail: 360ms;
  --page-enter-offset: 22px;
  --hover-scale-max: 1.04;

  --color-accent-blue: oklch(65.82% 0.1690 248.81);
  --color-accent-blue-hover: oklch(61.93% 0.1587 248.80);
  --color-accent-blue-pressed: oklch(57.94% 0.1468 248.61);
  --color-accent-green: oklch(72.27% 0.1920 149.58);
  --color-focus: var(--color-accent);
}

:root,
[data-theme="light"] {
  --color-paper: oklch(94.97% 0.0027 286.35);
  --color-paper-overlay: oklch(94.97% 0.0027 286.35 / 0.85);
  --color-panel: oklch(100% 0 0 / 0.50);
  --color-card: oklch(100% 0 0 / 0.70);
  --color-control: oklch(0% 0 0 / 0.07);
  --color-control-hover: oklch(82% 0.01 255 / 0.44);
  --color-rule: oklch(0% 0 0 / 0.03);
  --color-rule-strong: oklch(0% 0 0 / 0.14);
  --color-ink: oklch(27.81% 0.0296 256.85);
  --color-ink-2: oklch(0% 0 0 / 0.69);
  --color-ink-muted: oklch(0% 0 0 / 0.56);
  --color-accent: var(--color-accent-blue);
  --color-accent-hover: var(--color-accent-blue-hover);
  --color-accent-pressed: var(--color-accent-blue-pressed);
  --shadow-surface: 0 2px 15px oklch(0% 0 0 / 0.15);
  --backdrop-image-dim: 0.00;
}

[data-theme="dark"] {
  --color-paper: oklch(20.90% 0 0);
  --color-paper-overlay: oklch(20.90% 0 0 / 0.85);
  --color-panel: oklch(40.91% 0 0 / 0.50);
  --color-card: oklch(100% 0 0 / 0.094);
  --color-control: oklch(100% 0 0 / 0.125);
  --color-control-hover: oklch(100% 0 0 / 0.20);
  --color-rule: oklch(100% 0 0 / 0.086);
  --color-rule-strong: oklch(100% 0 0 / 0.26);
  --color-ink: oklch(96.72% 0 0);
  --color-ink-2: oklch(100% 0 0 / 0.69);
  --color-ink-muted: oklch(100% 0 0 / 0.56);
  --color-accent: var(--color-accent-green);
  --color-accent-hover: oklch(76% 0.18 149.58);
  --color-accent-pressed: oklch(66% 0.17 149.58);
  --shadow-surface: 0 2px 15px oklch(0% 0 0 / 0.15);
  --backdrop-image-dim: 0.40;
}
```

## Theme behaviour

- Appearance, accent, and backdrop are independent settings: `light | dark | system`, accent colour, and `solid | image | image-blur`.
- Theme changes colour and elevation only. Geometry, spacing, typography, density, and component placement do not shift.
- Light mode uses translucent white surfaces and faint dark rules.
- Dark mode uses lightness elevation: higher surfaces become slightly brighter; do not rely on heavier shadows.
- `Light / Azure` is the baseline light composition. `Dark / Forest` combines dark surfaces, green accent, and a dimmed forest image, but green is not mandatory for all dark themes.
- Image backdrops are environmental, never content. Apply tint and dimming until text and focus indicators meet contrast requirements.

## Component grammar

- Application shell · compact window chrome, 8 px outer radius, restrained caption buttons.
- Primary rail · icon-first, 46 px rows, 12 px selected surface, text appears only when expanded.
- Context panel · detached acrylic panel, 14 px radius, 24 px inset, 20 px semibold heading.
- Work header · back action, optional 28 px identity icon, title, search, and compact filters.
- Lists · 52–54 px rows, thumbnail + strong name + muted metadata; separators remain quieter than hover states.
- Home focus · one centred identity, one primary action, one subordinate settings link; generous negative space.
- Cards · use only for real grouping or floating state. Ordinary list rows are not boxed individually.
- Widgets · collapse the two-rail shell into one translucent surface; preserve tokens and action hierarchy rather than forcing desktop navigation into a small window.

## CTA voice

- Primary · accent fill, 8–9 px radius, 42 px height, semibold label, one per focal region.
- Secondary · neutral translucent or ghost surface with the same radius family.
- Tertiary · text or icon action; no decorative pill treatment.
- Accent footprint · 5–15% of the viewport. Do not colour every selected or interactive surface.

## Motion stance

- Conservative and state-explanatory: opacity plus transform wherever possible.
- Page entry · 22 px vertical offset to rest plus fade, 240 ms ease-out.
- Hover · 160 ms in and 220 ms out; communicate state through opacity or lightness.
- Rail expansion · 62 px to 176 px, 360 ms ease-in-out; label opacity follows the width transition.
- Avoid bounce and overshoot. Interactive imagery may scale no higher than `1.04`.
- Reduced-motion fallback · at most 150 ms opacity crossfade; remove spatial travel and scale.

## Accessibility

- Never suppress keyboard focus visuals. Use an immediate 2 px focus ring with at least 3:1 contrast.
- Provide complete default, hover, focus, active, disabled, loading, error, and success states.
- Keep primary text at WCAG AA contrast over both solid and image-backed surfaces.
- Re-evaluate contrast whenever the user changes the accent or background image.
- Interactive targets should be at least 44 × 44 px where touch or compact-window use is plausible.
- Do not encode selected, error, or success state by colour alone.

## Notes — do not carry over

- Do not copy BlockHelm, Minecraft, Microsoft, or Mojang names, logos, pixel avatars, grass-block imagery, icons, screenshots, or product copy.
- Do not reproduce the reference pixel-for-pixel. Recompose the Workbench structure for the new product's jobs and information architecture.
- Do not globally remove focus indicators, even if the reference implementation does.
- Do not reuse the reference avatar's `1.125` hover enlargement; it is too strong for a productivity interface.
- Do not blur every surface. Blur belongs to detached navigation, overlays, or image-backed controls where depth needs explanation.
- Do not tint every dark surface green. Keep surfaces neutral and reserve green or another accent for actions and state.
- Do not use pure black and pure white as the dominant paper and ink pair; preserve the softer tonal ladder.
- Do not force a permanent secondary rail into a widget-sized interface.

## Exports

This portable file is currently the canonical source. In an implementation
project, generate `tokens.css` first and treat it as the runtime source of
truth. Tailwind v4 `@theme`, DTCG `tokens.json`, shadcn/ui variables, WPF
resource dictionaries, or WinUI resources can be appended on request.
