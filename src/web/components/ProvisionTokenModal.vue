<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';
import { Button } from './ui/Button.vue';
import { Modal, ModalActions } from './ui/Modal.vue';
import { t, type Locale } from '../i18n.ts';

type ProvisionTokenModalProps = {
  locale: Locale;
  pluginName: string;
  working: boolean;
  result?: { ok: boolean; message: string };
  onSubmit: (username: string, password: string) => void;
  onClose: () => void;
};

/**
 * Admin-login dialog for one-click API token provisioning. Kept separate
 * from the connection card so operator credentials are never confused with
 * plugin settings. State-driven: fields are local, progress and outcome
 * come from the controller. The password is cleared on submit and dies with
 * the dialog; success hides the form so there is nothing left to edit.
 */
export const ProvisionTokenModal = defineVueComponent<ProvisionTokenModalProps>(
  ['locale', 'pluginName', 'working', 'result', 'onSubmit', 'onClose'],
  (props) => {
  const username = ref('');
  const password = ref('');
  // This dialog instance submitted at least once. Guards the result alert
  // so a reopened dialog never shows a stale outcome from a previous open.
  const attempted = ref(false);

  const submit = (): void => {
    if (props.working || username.value.trim().length === 0 || password.value.length === 0) return;
    attempted.value = true;
    props.onSubmit(username.value.trim(), password.value);
    password.value = '';
  };

  return () => {
    const succeeded = attempted.value && !props.working && props.result?.ok === true;
    const failed = attempted.value && !props.working && props.result && !props.result.ok;
    const canSubmit = !props.working && username.value.trim().length > 0 && password.value.length > 0;
    const locale = props.locale;
    return (
      <Modal
        title={t(locale, 'provisionTitle')}
        description={t(locale, 'provisionLead', { name: props.pluginName })}
        size="sm"
        onClose={props.onClose}
        closeOnBackdrop={!props.working}
        closeOnEscape={!props.working}
        footer={
          succeeded ? (
            <ModalActions>
              <Button variant="primary" onClick={props.onClose}>
                {t(locale, 'dialogClose')}
              </Button>
            </ModalActions>
          ) : (
            <ModalActions>
              <Button variant="soft" disabled={props.working} onClick={props.onClose}>
                {t(locale, 'dialogCancel')}
              </Button>
              <Button variant="primary" disabled={!canSubmit} onClick={submit}>
                {props.working ? t(locale, 'provisionWorking') : t(locale, 'provisionSubmit')}
              </Button>
            </ModalActions>
          )
        }
      >
        <div class="plg-form">
          {succeeded && props.result && (
            <div class="plg-alert plg-alert--ok" role="status">
              {props.result.message}
            </div>
          )}
          {failed && props.result && (
            <div class="plg-alert" role="status">
              {props.result.message}
            </div>
          )}
          {!succeeded && (
            <>
              <div class="plg-field">
                <label class="plg-label" for="prov-user">{t(locale, 'provisionAdminId')}</label>
                <input
                  id="prov-user"
                  class="plg-input"
                  type="text"
                  autocomplete="username"
                  value={username.value}
                  disabled={props.working}
                  onInput={(event) => { username.value = (event.currentTarget as HTMLInputElement).value; }}
                />
              </div>
              <div class="plg-field">
                <label class="plg-label" for="prov-pass">{t(locale, 'provisionAdminPassword')}</label>
                <input
                  id="prov-pass"
                  class="plg-input"
                  type="password"
                  autocomplete="current-password"
                  value={password.value}
                  disabled={props.working}
                  onInput={(event) => { password.value = (event.currentTarget as HTMLInputElement).value; }}
                  onKeydown={(event) => { if ((event as KeyboardEvent).key === 'Enter') submit(); }}
                />
              </div>
            </>
          )}
        </div>
      </Modal>
    );
  };
  },
);

export default ProvisionTokenModal;
</script>
