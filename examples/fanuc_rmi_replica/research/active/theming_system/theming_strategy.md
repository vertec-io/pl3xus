# Fanuc RMI Replica: Theming Strategy

This document outlines the proposed architecture for a dynamic theming system using Tailwind CSS and CSS variables, inspired by the shadcn/ui ecosystem.

## Goals

- **Decoupling**: Move theme-specific colors from component classes to a central configuration.
- **Dynamic Swapping**: Ability to change the entire application look by simply swapping a CSS file or updating variables.
- **Consistency**: Use a standard set of semantic tokens (primary, secondary, accent, etc.) across all components.
- **Opacity Support**: Ensure that themed colors can still use Tailwind's opacity modifiers (e.g., `primary/20`).

## Variable Convention

We will use HSL values for our CSS variables to allow Tailwind to inject opacity modifiers.

### Global Tokens

| Token | Description | Sample Usage |
|-------|-------------|--------------|
| `--background` | Main application background | Dashboard background |
| `--foreground` | Main text color | Headings, primary text |
| `--card` | Background for cards/widgets | Quick Commands, Jogging panel |
| `--card-foreground` | Text color within cards | Card titles, labels |
| `--popover` | Background for overlays/menus | Dropdowns, tooltips |
| `--primary` | Main accent color | Active states, status indicators |
| `--primary-foreground` | Text color on primary background | Button labels |
| `--secondary` | Subtle accent color | Secondary buttons, backgrounds |
| `--muted` | De-emphasized elements | Subtitles, disabled states |
| `--accent` | Highlighted elements | Hover states, active links |
| `--destructive` | Error/Danger actions | Stop buttons, error logs |
| `--border` | Default border color | Separators, card borders |
| `--input` | Input field borders | Text inputs, sliders |
| `--ring` | Focus ring color | Tab focus, active inputs |

## Naming Mappings (Audit Based)

Based on the current audit, here is how we will map existing hardcoded hex codes to semantic tokens:

| Hardcoded Hex | Proposed Token | Context |
|---------------|----------------|---------|
| `#0a0a0a` | `--background` | Main background |
| `#111111` | `--card` | Component containers |
| `#1a1a1a` | `--popover` / `--secondary` | Darker widgets/alternates |
| `#00d9ff` | `--primary` | Industrial Cyan accent |
| `#22c55e` | `--success` (custom) | Green status / "Running" |
| `#f59e0b` | `--warning` (custom) | Amber status / "Warning" |
| `#ff4444` | `--destructive` | Red status / "Emergency" |
| `#888888` | `--muted-foreground` | De-emphasized labels |
| `#ffffff08` | `--border` | Faint borders |

## Implementation Plan (Research Phase)

1. **Define Variables**: Create `globals.css` with `:root` variables using HSL format.
2. **Tailwind Config**: Extend the colors in `tailwind.config.js` to point to these variables.
3. **Alternate Themes**: Create additional CSS files (e.g., `ocean_theme.css`) that redefine the same variables.

## How to Test Swapping

To test a new theme, one would simply replace the content of `globals.css` (or the linked stylesheet) with the contents of the alternate theme file. Because all components will eventually use classes like `bg-primary`, `border-border`, and `text-foreground`, they will automatically update based on the variables.
