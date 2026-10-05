export interface ViewportWindowRequest {
  /** Inclusive absolute item offset requested from the app-owned source. */
  readonly start: number;
  /** Exclusive absolute item offset requested from the app-owned source. */
  readonly end: number;
  /** Bounded app-declared total; no records accompany the request. */
  readonly total: number;
  readonly component: HTMLElement;
  readonly scroller: HTMLElement;
  /** Aborted when the specimen leaves its owning root or its installer is cleaned up. */
  readonly signal: AbortSignal;
}

export interface ViewportNavigationRequest {
  readonly component: HTMLElement;
  /** The admitted native control; href/action/value remain app-owned query intent. */
  readonly control: HTMLElement;
  readonly signal: AbortSignal;
  /** Present only when the component declares a valid windowed extent. */
  readonly window?: Readonly<{ start: number; end: number; total: number }>;
}

export type DataViewportAdapter =
  | {
      window(request: ViewportWindowRequest): unknown | Promise<unknown>;
      navigate?(request: ViewportNavigationRequest): unknown | Promise<unknown>;
    }
  | {
      window?: (request: ViewportWindowRequest) => unknown | Promise<unknown>;
      navigate(request: ViewportNavigationRequest): unknown | Promise<unknown>;
    };

/**
 * Observe app-supplied ports. Native navigation is never prevented. Mark admitted
 * links/buttons/forms with data-cui-viewport-navigation to expose their control context.
 */
export declare function install(root: ParentNode & EventTarget, adapter?: DataViewportAdapter): () => void;
