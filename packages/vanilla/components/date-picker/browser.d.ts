/** Browser draft values emitted after associated native inputs have been synchronized. */
export interface PickerSelectionDetail {
  readonly mode: 'single' | 'range';
  /** Strict Gregorian YYYY-MM-DD, or null for an empty native draft. */
  readonly start: string | null;
  readonly end: string | null;
  readonly complete: boolean;
}
/** Emits bubbling cui:date-picker-changed CustomEvent<PickerSelectionDetail>.
 * The app owns form submission, authoritative validation, and command transport.
 */
export declare function install(root?: Document | Element): () => void;
