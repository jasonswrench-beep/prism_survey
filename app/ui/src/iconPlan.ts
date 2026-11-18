// Every interface icon Prism Survey places, by file name, from the icon brief (design/icons/Prism Survey Icon List.md).
// The window may name only these. A name whose SVG file is in assets/icons draws; a name not drawn yet draws nothing,
// so its button shows its words alone until the file arrives. The test in iconPlan.test.ts keeps this list and the
// brief in step.

/** Copied unchanged from PrismQDA (assets/icons/regular there), so the two tools read as one family. */
export const REUSED_ICONS = [
  'cmd-import', 'cmd-search', 'set-about', 'src-folder',
  'st-error', 'st-info', 'st-save-failed', 'st-saved', 'st-saving', 'st-success', 'st-warning',
  'ui-check', 'ui-chevron-down', 'ui-chevron-right', 'ui-close', 'ui-copy', 'ui-delete', 'ui-edit', 'ui-filter',
  'ui-plus', 'ui-save', 'ui-more', 'ui-help', 'ui-refresh', 'ui-external', 'ui-eye', 'ui-sort', 'ui-drag', 'ui-pin',
  'ui-pane-collapse', 'ui-command',
] as const;

/** Drawn for Prism Survey, not yet supplied. Batch 1 first, then Batch 2, in the brief's order. */
export const NEW_ICONS = [
  // Batch 1
  'fmt-blank-line', 'fmt-box-grid', 'fmt-semantic',
  'cmd-find-article', 'view-directory', 'cmd-open', 'cmd-save-as', 'cmd-variant', 'exp-respondent', 'exp-researcher',
  'cmd-benchmark', 'study-new', 'view-study', 'ui-clock',
  'tag-rename', 'study-add', 'lib-variant',
  'instr-house', 'item-split', 'item-merge', 'item-lead-in', 'item-trim', 'item-reversed', 'item-reverse-suggested',
  'item-changed', 'sub-add', 'item-clear',
  'chk-selfcheck', 'chk-fix', 'chk-dismiss', 'chk-published', 'chk-corrected', 'chk-reopen', 'chk-locked',
  'art-candidate', 'art-outline', 'art-incomplete',
  // Batch 2
  'study-page-break', 'study-transition', 'study-demographics', 'study-refresh', 'study-suggest', 'study-label',
  'study-numbering', 'study-references',
  'dir-ref-checked', 'dir-ref-check', 'dir-chapter',
  'var-replace', 'var-accept-all', 'var-flag', 'var-carry', 'var-leave',
  'lic-open', 'lic-permissioned', 'lic-local', 'lic-unknown',
] as const;

/**
 * The brief's two file glyphs (file-scale, file-study) are full-color pieces for Windows' file list, not interface
 * icons, so they are not drawn through this module and are not listed here.
 */
export type IconName = (typeof REUSED_ICONS)[number] | (typeof NEW_ICONS)[number];
