// The scale record as the engine sends it (crates/prismsurvey-core/src/model.rs). Field names are the .pscale names.

export type Format = 'likertBlankLine' | 'likertBoxGrid' | 'semanticDifferential';
export type AnchorSet = 'agreement' | 'frequency' | 'custom';
export type LabelMode = 'every' | 'endsOnly';
export type InstructionsOrigin = 'house' | 'verbatim' | 'edited';
export type ReverseState = 'notReversed' | 'suggested' | 'confirmed';
export type ScoreMethod = 'sum' | 'mean';
export type TotalFrom = 'items' | 'subscales';
export type LicenceStatus = 'open' | 'permissioned' | 'localOnly' | 'unknown';
export type SourcePlacement = 'underScale' | 'references' | 'none';

export const FORMAT_LABEL: Record<Format, string> = {
  likertBlankLine: 'Likert blank-line',
  likertBoxGrid: 'Likert box grid',
  semanticDifferential: 'Semantic differential',
};

export interface ResponseScale {
  points: number;
  anchorSet: AnchorSet;
  labelMode: LabelMode;
  labels: (string | null)[];
  abbreviations: string[];
  values: number[];
}

export interface Item {
  number: number;
  text: string;
  left?: string;
  right?: string;
  reverse: { state: ReverseState; reason?: string };
  sourceText?: string;
  /// Text printed just above this item, as the prompt for the block it starts.
  leadIn?: string;
}

export type ItemSelector =
  | { kind: 'list'; numbers: number[] }
  | { kind: 'odd' }
  | { kind: 'even' }
  | { kind: 'block'; first: number; last: number };

export interface Subscale {
  name: string;
  items: ItemSelector;
  otherNames?: string[];
}

export interface Citation {
  apa: string;
  doi?: string;
  referenceManagerId?: string;
}

export interface DevelopmentItem {
  number: number;
  text: string;
  loadings: (number | null)[];
  markedKept?: boolean;
  finalNumber?: number;
  matchScore?: number;
}

export interface Provenance {
  field: string;
  file: string;
  page?: number;
  line?: number;
  /// Left, top, right, bottom in points on the page, for article fields.
  region?: [number, number, number, number];
  confidence: number;
  rule?: string;
}

export interface Scale {
  pscaleVersion: number;
  id: string;
  title: string;
  shortName?: string;
  countSuffix: boolean;
  tags: string[];
  format: Format;
  response: ResponseScale;
  instructions: { text: string; origin: InstructionsOrigin; houseStem?: string };
  items: Item[];
  recode: { rule: { kind: 'reflect' } | { kind: 'table'; pairs: [number, number][] }; stated?: { numbers: number[]; text: string } };
  subscales: Subscale[];
  scoring: {
    subscaleMethod: ScoreMethod;
    totalMethod: ScoreMethod;
    totalFrom: TotalFrom;
    hasTotal: boolean;
    totalName?: string;
    otherTotalNames?: string[];
    stated?: string[];
    interpretation?: string[];
  };
  scoreRange: { stated?: [number, number]; statedText?: string; computed?: [number, number] };
  source?: Citation;
  adaptedFrom?: Citation;
  sourcePlacement: SourcePlacement;
  licence: { status: LicenceStatus; note?: string };
  development?: { factors: string[]; factorLabels?: string[]; items: DevelopmentItem[]; notes?: string[] };
  provenance: Provenance[];
  history: { version: number; savedAt: string; summary: string }[];
  unparsed?: { line: number; text: string }[];
  checkDecisions?: { key: string; status: 'accepted' | 'dismissed'; reason: string; decidedAt: string }[];
  directoryEntry?: string;
  sourceNotes?: SourceNote[];
  variantOf?: {
    parentId: string;
    parentTitle: string;
    parentVersion: number;
    oldTarget: string;
    newTarget: string;
    context?: string;
    replacements?: { from: string; to: string }[];
    itemMap: { number: number; originalNumber: number; originalText: string; flagged?: string }[];
    locked?: unknown;
    declined?: string[];
  };
}

export interface SourceNote {
  key: string;
  subject: string;
  items?: number[];
  original: string;
  page?: string;
  handling: 'kept' | 'corrected';
  change?: string;
  notedAt: string;
}

export interface ImportReport {
  scale: Scale;
  file: string;
  part: number;
  isTemplate: boolean;
  notes: string[];
}

export interface FitFinding {
  words: string;
  message: string;
}

export interface ChangeOutcome {
  instructionsRewritten: boolean;
  findings: FitFinding[];
}

export type Fix =
  | { kind: 'setReversed'; item: number; reversed: boolean }
  | { kind: 'recodeRepeat'; number: number; with: number | null }
  | { kind: 'syncTitleCount' }
  | { kind: 'useComputedRange' }
  | { kind: 'clearSource' }
  | { kind: 'useHouseInstructions' }
  | { kind: 'useTotalName'; name: string }
  | { kind: 'restoreLockedStructure' };

export interface Finding {
  key: string;
  check: string;
  checkLabel: string;
  severity: 'warning' | 'note';
  message: string;
  items: number[];
  fixes?: Fix[];
  fixLabels?: string[];
  status: 'open' | 'accepted' | 'dismissed' | 'published';
  reason?: string;
}

export interface NoteLine {
  key: string;
  subject: string;
  items: number[];
  handling: 'kept' | 'corrected';
  text: string;
}

export interface Check {
  findings: Finding[];
  sourceNotes: NoteLine[];
  computedRange: [number, number] | null;
  instructionFindings: FitFinding[];
  forcedChoice: boolean;
  titleCountMismatch: [number, number] | null;
  filledItems: number;
}

export interface LibraryEntry {
  path: string;
  title: string;
  format: Format;
  items: number;
  tags: string[];
  updated: string | null;
  error: string | null;
  variantChain?: string[];
}

export type ParentChange =
  | { kind: 'itemText'; key: string; number: number; was: string; now: string; proposed: string }
  | { kind: 'reverse'; key: string; number: number; reversed: boolean }
  | { kind: 'sourceNote'; key: string; note: SourceNote; text: string }
  | { kind: 'structure'; key: string; what: string[] }
  | { kind: 'itemCount'; key: string; was: number; now: number };

export interface Settings {
  libraryFolder: string;
}

export interface TagCount {
  tag: string;
  count: number;
}
