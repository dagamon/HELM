# Settings and Theme

## What settings control

Settings let you adjust how HELM looks and behaves. The Appearance tab picks the dashboard theme; General and Updates hold browser-local preferences and self-update controls.

## Themes

Themes are JSON files in the `themes/` folder of the HELM repository. Each file defines the full dashboard palette (`colors`) plus a set of panel colors (`panels`) that cards may use. The Appearance tab only lets you pick one of these ready themes — there is no in-app color editor.

The active theme is stored server-side (shared across browsers) and cached locally for instant paint.

## Custom themes

Create a custom theme by copying an existing file in `themes/`, then changing `name`, `label`, `colors`, and the `panels` palette. Reload the dashboard and it appears in Settings → Appearance. Keep contrast high enough that statuses, buttons, and logs remain readable.

## Panel colors

Service and stack cards on the dashboard can be tinted individually: hover a card and use the palette button to pick one of the active theme's panel colors. The choice is stored as a palette key, so switching themes remaps every card to the new theme's version of that color.

## Adding a service to a stack

Drag a service card onto a stack card on the dashboard to add the service to that stack.
