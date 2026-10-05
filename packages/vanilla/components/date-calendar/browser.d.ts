/** Strict Gregorian YYYY-MM-DD in years 0001..9999; runtime validation remains mandatory. */
export type ISODate = string;
/** Browser draft selection, not an authorization, domain update, or server validation. */
export interface CalendarSelectionDetail {
  readonly mode: 'single' | 'range';
  readonly start: ISODate;
  readonly end: ISODate | null;
  readonly complete: boolean;
}
/** Emits bubbling cui:date-calendar-changed CustomEvent<CalendarSelectionDetail>. */
export declare function install(root?: Document | Element): () => void;
