import { ICON_DEFS } from './assets/icons.generated';
import type { IconName } from './iconPlan';

export type { IconName };

/**
 * One icon from assets/icons, drawn as ordinary SVG elements that take their color from the text around them, so dark
 * mode and Windows high contrast need no extra rules (PrismQDA's decision 41). It is decorative: the button or label it
 * sits in carries the accessible name, so an icon-only button still needs its own aria-label.
 *
 * An icon the brief plans but whose file is not in assets/icons yet draws nothing, and its button shows its words alone.
 * `turn` rotates a drawn arrow or chevron: the set draws one direction only.
 */
export function Icon({ name, size = 18, turn, className }: { name: IconName; size?: number; turn?: 90 | 180 | 270; className?: string }) {
  const def = ICON_DEFS[name];
  if (!def) return null;
  const classes = ['icon', turn ? `turn-${turn}` : '', className ?? ''].filter(Boolean).join(' ');
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox={def.viewBox}
      width={size}
      height={size}
      className={classes}
      aria-hidden="true"
      focusable="false"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {def.paths.map((p, i) => (
        <path key={i} d={p.d} fillRule={p.fillRule} fill={p.fill} stroke={p.stroke} strokeDasharray={p.dash} />
      ))}
    </svg>
  );
}

/** True when the icon's file is in assets/icons, for the few places that show a stand-in until it is. */
export function hasIcon(name: IconName): boolean {
  return name in ICON_DEFS;
}
