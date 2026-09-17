<script lang="tsx">
import { fieldMessageIds } from './field-logic.ts';

export type FieldMessageProps = {
  /** Control id the message ids derive from (see `fieldMessageIds`). */
  id: string;
  /** Visible help under the control. Hidden while an error shows. */
  description?: string;
  error?: string;
};

/**
 * The one inline message line: error wins over help text. Ids match what the
 * control references via `aria-describedby` / `aria-errormessage`, so both
 * sides must keep deriving them from `fieldMessageIds`.
 */
export function FieldMessage({ id, description, error }: FieldMessageProps) {
  if (error) {
    return (
      <span id={fieldMessageIds(id).errorId} class="field-message field-message--error">
        {error}
      </span>
    );
  }
  if (description) {
    return (
      <span id={fieldMessageIds(id).descriptionId} class="field-message field-message--hint">
        {description}
      </span>
    );
  }
  return null;
}

export default FieldMessage;
</script>
