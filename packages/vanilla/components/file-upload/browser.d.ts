export interface FileSelectionRequest {
  readonly component: HTMLElement;
  readonly files: readonly File[];
  /** Original native input; it remains the form control for no-JavaScript submission. */
  readonly input: HTMLInputElement;
  /** Aborted when this component leaves its owning root or its installer is cleaned up. */
  readonly signal: AbortSignal;
}

export interface FileUploadAdapter {
  /** Receives native browser File references, never an upload-completion assertion. */
  selection(request: FileSelectionRequest): unknown | Promise<unknown>;
  /** Optional notification when the native chooser is cancelled; files is empty. */
  cancel?(request: FileSelectionRequest): unknown | Promise<unknown>;
}

/** No adapter means no enhancement; the native input and its FileList are untouched. */
export declare function install(root: ParentNode & EventTarget, adapter?: FileUploadAdapter): () => void;
