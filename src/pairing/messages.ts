import type { Locale } from '../web/shared/i18n/supported-locales';

/**
 * Every user-visible string of the adapter's pairing page. The catalogs are
 * typed so a missing key or locale fails `vue-tsc` instead of rendering blank.
 */
const en = {
  documentTitle: 'TikTools pairing',
  title: 'TikTools local browser',
  intro: 'Enter the six-digit, one-use pairing code printed by the adapter. It expires after five minutes. The terminal one-click join link fills it in automatically.',
  label: 'Pairing code',
  connect: 'Connect',
  renew: 'Request new code',
  invalidCode: 'Incorrect or expired code. Try again or request a new code.',
  adapterUnavailable: 'Adapter unavailable. Check the terminal where the adapter is running.',
  signingIn: 'Signing in with the terminal link…',
  requesting: 'Requesting a new code…',
  newCodePrinted: 'New code printed in the terminal. Enter it here within five minutes.',
  waitSeconds: 'Please wait {seconds} seconds before requesting another code.',
  lockedOut: 'Too many attempts. Restart the adapter to pair again.',
  alreadyPaired: 'Already paired. Reload this page, or restart the adapter to pair another browser.',
  renewFailed: 'Could not request a code. Check the terminal where the adapter is running.',
} as const;

export type MessageKey = keyof typeof en;
type Catalog = Record<MessageKey, string>;

const es: Catalog = {
  documentTitle: 'Vinculación de TikTools',
  title: 'Navegador local de TikTools',
  intro: 'Introduce el código de vinculación de seis dígitos, de un solo uso, que muestra el adaptador. Caduca a los cinco minutos. El enlace de un clic de la terminal lo rellena automáticamente.',
  label: 'Código de vinculación',
  connect: 'Conectar',
  renew: 'Solicitar código nuevo',
  invalidCode: 'Código incorrecto o caducado. Inténtalo de nuevo o solicita un código nuevo.',
  adapterUnavailable: 'Adaptador no disponible. Revisa la terminal donde se está ejecutando el adaptador.',
  signingIn: 'Iniciando sesión con el enlace de la terminal…',
  requesting: 'Solicitando un código nuevo…',
  newCodePrinted: 'Código nuevo impreso en la terminal. Introdúcelo aquí en los próximos cinco minutos.',
  waitSeconds: 'Espera {seconds} segundos antes de solicitar otro código.',
  lockedOut: 'Demasiados intentos. Reinicia el adaptador para volver a vincular.',
  alreadyPaired: 'Ya está vinculado. Recarga esta página o reinicia el adaptador para vincular otro navegador.',
  renewFailed: 'No se pudo solicitar un código. Revisa la terminal donde se está ejecutando el adaptador.',
};

const pt: Catalog = {
  documentTitle: 'Pareamento do TikTools',
  title: 'Navegador local do TikTools',
  intro: 'Digite o código de pareamento de seis dígitos, de uso único, exibido pelo adaptador. Ele expira em cinco minutos. O link de entrada com um clique do terminal preenche o código automaticamente.',
  label: 'Código de pareamento',
  connect: 'Conectar',
  renew: 'Solicitar novo código',
  invalidCode: 'Código incorreto ou expirado. Tente novamente ou solicite um novo código.',
  adapterUnavailable: 'Adaptador indisponível. Verifique o terminal onde o adaptador está em execução.',
  signingIn: 'Entrando com o link do terminal…',
  requesting: 'Solicitando um novo código…',
  newCodePrinted: 'Novo código exibido no terminal. Digite-o aqui em até cinco minutos.',
  waitSeconds: 'Aguarde {seconds} segundos antes de solicitar outro código.',
  lockedOut: 'Muitas tentativas. Reinicie o adaptador para parear novamente.',
  alreadyPaired: 'Já pareado. Recarregue esta página ou reinicie o adaptador para parear outro navegador.',
  renewFailed: 'Não foi possível solicitar um código. Verifique o terminal onde o adaptador está em execução.',
};

const fr: Catalog = {
  documentTitle: 'Appairage TikTools',
  title: 'Navigateur local TikTools',
  intro: "Saisissez le code d'appairage à six chiffres, à usage unique, affiché par l'adaptateur. Il expire au bout de cinq minutes. Le lien de connexion en un clic du terminal le remplit automatiquement.",
  label: "Code d'appairage",
  connect: 'Se connecter',
  renew: 'Demander un nouveau code',
  invalidCode: 'Code incorrect ou expiré. Réessayez ou demandez un nouveau code.',
  adapterUnavailable: "Adaptateur indisponible. Vérifiez le terminal où l'adaptateur est en cours d'exécution.",
  signingIn: 'Connexion avec le lien du terminal…',
  requesting: "Demande d'un nouveau code…",
  newCodePrinted: 'Nouveau code affiché dans le terminal. Saisissez-le ici dans les cinq minutes.',
  waitSeconds: 'Patientez {seconds} secondes avant de demander un autre code.',
  lockedOut: "Trop de tentatives. Redémarrez l'adaptateur pour vous appairer à nouveau.",
  alreadyPaired: "Déjà appairé. Rechargez cette page ou redémarrez l'adaptateur pour appairer un autre navigateur.",
  renewFailed: "Impossible de demander un code. Vérifiez le terminal où l'adaptateur est en cours d'exécution.",
};

const zh: Catalog = {
  documentTitle: 'TikTools 配对',
  title: 'TikTools 本地浏览器',
  intro: '输入适配器显示的六位一次性配对码。配对码五分钟后过期。终端中的一键加入链接会自动填入配对码。',
  label: '配对码',
  connect: '连接',
  renew: '请求新配对码',
  invalidCode: '配对码错误或已过期。请重试或请求新的配对码。',
  adapterUnavailable: '适配器不可用。请检查运行适配器的终端。',
  signingIn: '正在通过终端链接登录…',
  requesting: '正在请求新配对码…',
  newCodePrinted: '新配对码已显示在终端中。请在五分钟内在此输入。',
  waitSeconds: '请等待 {seconds} 秒后再请求另一个配对码。',
  lockedOut: '尝试次数过多。请重启适配器后重新配对。',
  alreadyPaired: '已配对。请刷新此页面，或重启适配器以配对另一个浏览器。',
  renewFailed: '无法请求配对码。请检查运行适配器的终端。',
};

const ja: Catalog = {
  documentTitle: 'TikTools ペアリング',
  title: 'TikTools ローカルブラウザ',
  intro: 'アダプターに表示された、一度だけ使える6桁のペアリングコードを入力してください。コードは5分で期限切れになります。ターミナルのワンクリック参加リンクを使うと自動で入力されます。',
  label: 'ペアリングコード',
  connect: '接続',
  renew: '新しいコードを要求',
  invalidCode: 'コードが正しくないか、期限切れです。もう一度入力するか、新しいコードを要求してください。',
  adapterUnavailable: 'アダプターに接続できません。アダプターを実行しているターミナルを確認してください。',
  signingIn: 'ターミナルのリンクでサインインしています…',
  requesting: '新しいコードを要求しています…',
  newCodePrinted: '新しいコードがターミナルに表示されました。5分以内にここに入力してください。',
  waitSeconds: '別のコードを要求するまで {seconds} 秒お待ちください。',
  lockedOut: '試行回数が多すぎます。アダプターを再起動して、もう一度ペアリングしてください。',
  alreadyPaired: 'ペアリング済みです。このページを再読み込みするか、アダプターを再起動して別のブラウザをペアリングしてください。',
  renewFailed: 'コードを要求できませんでした。アダプターを実行しているターミナルを確認してください。',
};

const catalogs: Record<Locale, Catalog> = { en, es, pt, fr, zh, ja };

export type Translate = (key: MessageKey, params?: Record<string, string | number>) => string;

export function translatorFor(locale: Locale): Translate {
  const catalog = catalogs[locale];
  return (key, params = {}) =>
    catalog[key].replace(/\{(\w+)\}/g, (placeholder, name: string) =>
      name in params ? String(params[name]) : placeholder,
    );
}
