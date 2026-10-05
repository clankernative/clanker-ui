export interface ConfirmDialogSubmitRequest {
  /** The original form associated with the confirmation trigger. */
  readonly form: HTMLFormElement;
  /** The original submit button, preserving its name/value intent for the app adapter. */
  readonly submitter: HTMLButtonElement;
  /** Aborted when the dialog is canceled/closed or this installation is disposed. */
  readonly signal: AbortSignal;
}

/** App-owned boundary for handling confirmed intent; it does not imply authorization or success. */
export interface ConfirmDialogInstallOptions {
  submit?(request: ConfirmDialogSubmitRequest): void | Promise<void>;
  /** Accessible local feedback shown when the app adapter rejects; defaults to English. Plain text, 1–200 characters. */
  failureLabel?: string;
}

/**
 * Installs enhancement on the supplied root. Without a custom port, supported browsers use the
 * form's native requestSubmit(originalSubmitter); otherwise the original native button fallback remains.
 * The returned disposer is idempotent and aborts pending port work without restoring focus on teardown.
 */
export declare function install(
  root?: Document | Element,
  options?: ConfirmDialogInstallOptions,
): () => void;
