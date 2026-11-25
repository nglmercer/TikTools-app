import type { TourCatalog } from './index.ts';
export default {
  controls: { next: 'Siguiente', previous: 'Anterior', done: 'Terminar guía', close: 'Cerrar guía', progress: '{{current}} de {{total}}' },
  connection: {
    form: { title: 'Elige un creador', description: 'Introduce su nombre y revisa el estado. La guía nunca conecta ni lee tu cookie de sesión.' },
    explorer: { title: 'Explora salas LIVE', description: 'Actualiza o elige una sala por tu cuenta. La conexión demo es simulada; el acceso nativo depende del host y de TikTok.' },
    plugins: { title: 'Comprueba integraciones', description: 'Revisa cada conexión instalada y su salud antes de habilitar efectos. Abrir la guía no contacta servicios de plugins.' },
  },
  plugin: {
    catalog: { title: 'Instalados y Explore', description: 'Revisa paquetes instalados o explora entradas compatibles. Los plugins nativos y de proceso mantienen los permisos del usuario en el sistema.' },
    search: { title: 'Busca un paquete', description: 'Busca e inspecciona editor, versión y requisitos. Un catálogo vacío y un backend inaccesible son estados diferentes.' },
    install: { title: 'Revisa la instalación', description: 'Instalar exige revisar firmas y permisos. La guía no instala, aprueba permisos nuevos ni ejecuta acciones de prueba.' },
  },
  overlay: {
    gateway: { title: 'Comprueba el gateway', description: 'La salida en vivo necesita el plugin event-gateway y credencial de widget. Una vista previa local no demuestra conexión OBS.' },
    preview: { title: 'Previsualiza localmente', description: 'La vista utiliza el mismo renderizador con eventos de muestra. No necesita LIVE real ni envía alertas a OBS.' },
    design: { title: 'Revisa el diseño', description: 'Edita y guarda intencionalmente; después copia una URL actualizada para OBS. Changes de Overlays sigue siendo un seguimiento sin integrar.' },
  },
  points: {
    rules: { title: 'Revisa reglas de puntos', description: 'Inspecciona los valores antes de modificarlos. Algunos controles se bloquean durante una sesión LIVE conectada u ocupada.' },
    leaderboard: { title: 'Inspecciona puntos confirmados', description: 'Busca u ordena espectadores. Actualizar la vista no ajusta ni reinicia puntos, tampoco repite eventos LIVE.' },
  },
  automationEvents: {
    profiles: { title: 'Elige el perfil de trabajo', description: 'Los eventos y acciones pertenecen a un perfil. Revisa el perfil que realmente usas antes de cambiar cualquier regla.' },
    eventsTab: { title: 'Abre la pestaña Eventos', description: 'Los eventos son las reglas: cada uno vincula un disparador LIVE con las acciones que ejecuta. La pestaña Eventos es la vista por defecto.' },
    events: { title: 'Lee la tabla de eventos', description: 'Cada fila muestra el disparador, sus condiciones y las acciones vinculadas. Mantén los eventos nuevos desactivados hasta revisar su definición.' },
    editor: { title: 'Edita un evento a la vez', description: 'El editor define el disparador, los filtros, el cooldown y las acciones vinculadas. Guardar es explícito; nada se ejecuta durante la guía.' },
    runs: { title: 'Verifica con el historial', description: 'Las ejecuciones recientes muestran qué se disparó, qué se omitió y por qué. Confirma cooldowns y filtros aquí antes de activar un evento.' },
  },
  automationActions: {
    actionsTab: { title: 'Abre la pestaña Acciones', description: 'Las acciones son efectos reutilizables compartidos por eventos. Abre la pestaña Acciones para revisarlas independientemente de cualquier disparador.' },
    actions: { title: 'Lee la tabla de acciones', description: 'Cada fila es una acción reutilizable con su tipo y destino. Editar una acción actualiza todos los eventos que la vinculan.' },
    editor: { title: 'Configura la acción', description: 'El editor define qué hace el efecto: texto, sonido, overlay o puntos. Las acciones de prueba nunca se ejecutan durante la guía.' },
    templates: { title: 'Empieza con una plantilla', description: 'Las plantillas de reglas previsualizan una configuración reutilizable. Importar y activar requieren revisión propia; la guía no hace ninguna de las dos cosas.' },
  },
  automationVariables: {
    variablesTab: { title: 'Abre la pestaña Variables', description: 'Las variables viven con las automatizaciones. Abre la pestaña Variables para revisar valores compartidos por eventos y acciones.' },
    values: { title: 'Revisa valores compartidos', description: 'Los secretos permanecen de solo escritura y nunca se muestran. Cambiar un valor afecta a cada evento y acción que lo referencia.' },
    review: { title: 'Revisa Cambios antes de sincronizar', description: 'Los valores locales y de Cloud se comparan en Cambios. Revisa las diferencias antes de aplicar cualquiera de las dos direcciones.' },
  },
  changes: {
    library: { title: 'Elige los registros', description: 'Eventos, acciones y variables tienen límites de revisión separados. Conserva los candidatos sin guardar antes de revisar otro estado.' },
    review: { title: 'Revisa Changes', description: 'Changes de Automations es local. Variables revisadas utiliza comprobaciones nativas de revisión; esta revisión no sube valores crudos.' },
    profiles: { title: 'Mantén la recuperación explícita', description: 'Resuelve una confirmación incierta mediante su diario original. Cerrar la guía no completa tareas ni reintenta mutaciones.' },
  },
  diagnostics: {
    log: { title: 'Inspecciona el historial acotado', description: 'Filtra origen, nivel o texto y revisa repeticiones. Comprueba exportaciones ocultadas antes de compartir; el historial dura solo la sesión.' },
    back: { title: 'Revisa evidencia y recupera', description: 'Copia o exporta evidencia revisada y vuelve al flujo original ante conflictos, sesiones vencidas u órdenes inciertas. Borrar registros no repara el error.' },
  },
  feed: {
    view: { title: 'Elige la vista del feed', description: 'Feed en vivo muestra chat, regalos y seguidores como tarjetas; cambia a Lista para ver el registro compacto.' },
    filters: { title: 'Filtra la actividad', description: 'Filtra por tipo o busca por espectador y mensaje. Los filtros también afectan a las tarjetas del feed.' },
    stage: { title: 'Lee el muro en vivo', description: 'Los mensajes importantes permanecen. Los me gusta y las entradas aparecen un momento como reacciones y luego se desvanecen.' },
    controls: { title: 'Pausar, limpiar e inspeccionar', description: 'Pausa el desplazamiento para leer con calma, limpia el historial guardado o abre el inspector para ver detalles.' },
  },
  backup: {
    plan: { title: 'Conoce tus límites', description: 'Cada plan tiene su propio límite de almacenamiento y de copias. Estos medidores muestran cuánto usas.' },
    tools: { title: 'Nube o este PC', description: 'Cambia entre guardar en tu cuenta en la nube o trabajar con archivos en este PC: exporta tus datos o sube un archivo que ya tengas.' },
    create: { title: 'Guarda una copia', description: 'Ponle nombre a una copia y guárdala en tu cuenta. Incluye reglas, perfiles, ajustes y archivos en la nube, nunca secretos ni cookies.' },
    list: { title: 'Descarga, restaura o elimina', description: 'Cada copia guardada se puede descargar, restaurar en cualquier dispositivo con esta cuenta o eliminar. Restaurar siempre muestra antes qué cambiará.' },
  },
  analytics: {
    range: { title: "Elige el rango de fechas", description: "Elige un preset o fechas personalizadas. El resumen se recarga para el creador activo; cambiar el rango nunca modifica los eventos guardados." },
    metrics: { title: "Elige la métrica", description: "Las tarjetas comparan chats, regalos, likes y entradas. La métrica seleccionada controla la gráfica sin modificar los datos." },
    activity: { title: "Lee la actividad", description: "Usa la gráfica para detectar días y horas activos. Un periodo vacío puede significar que no hubo LIVE o no hubo eventos coincidentes." },
    facts: { title: "Revisa los datos clave", description: "Los totales y sesiones ofrecen una vista compacta antes de comparar espectadores concretos." },
    contributors: { title: "Busca contribuyentes", description: "Ordena o busca en la tabla para encontrar los espectadores más activos. Es una clasificación de solo lectura." },
  },
  settings: {
    sections: { title: "Elige una sección", description: "General contiene preferencias de la aplicación. Cuenta contiene perfil Cloud, plan y copias de seguridad." },
    language: { title: "Cambia el idioma", description: "El idioma se aplica inmediatamente a la interfaz. No cambia datos del LIVE ni el comportamiento de automatizaciones." },
    review: { title: "Elige el modo de revisión", description: "El modo de revisión controla cuánta confirmación piden los flujos sensibles de sincronización. Los modos más seguros exigen revisión explícita." },
    updater: { title: "Busca actualizaciones", description: "Usa la tarjeta del actualizador para ver el canal y comprobar actualizaciones. Si no está disponible, usa la ruta de recuperación documentada." },
  },
  overlayObs: {
    gateway: { title: "Empieza por el gateway", description: "Los widgets de OBS reciben eventos mediante el gateway local. Confirma que está activo antes de copiar cualquier URL." },
    widget: { title: "Elige el widget", description: "Elige la fuente de follows, regalos, chat, likes o ranking que quieres colocar en OBS." },
    preview: { title: "Prueba con la vista previa", description: "Reproduce eventos de ejemplo y ajusta el diseño localmente. La vista previa no demuestra que OBS esté conectado." },
    url: { title: "Copia la URL para OBS", description: "Copia la URL solo cuando el gateway esté listo y pégala en una Browser Source de OBS con el tamaño deseado." },
    design: { title: "Ajusta animación y apariencia", description: "El editor incluye estilo de animación, duración, curva y retardo. Guarda antes de copiar una URL actualizada." },
  },
  cloudOverlays: {
    open: { title: "Abre la biblioteca Cloud", description: "Cuando los overlays Cloud estén disponibles, explora documentos compartidos desde el botón de biblioteca Cloud." },
    widgets: { title: "Selecciona el destino local", description: "Elige qué widget local debe recibir el diseño Cloud seleccionado antes de aplicar nada." },
    preview: { title: "Previsualiza antes de publicar", description: "Usa la vista previa local para comparar el diseño. Publicar o sobrescribir una copia Cloud sigue siendo una acción explícita." },
    url: { title: "Copia la fuente correcta", description: "Las URLs del gateway local y las URLs Cloud resuelven casos distintos. Verifica cuál debe usar tu escena de OBS." },
  },
  rankings: {
    tabs: { title: "Rankings o competiciones", description: "Los rankings acumulan puntuaciones. Las competiciones comparan rankings seleccionados durante un evento limitado." },
    list: { title: "Elige o crea un ranking", description: "La lista muestra métrica, modo y estado. Crea un ranking solo después de decidir qué acción del espectador puntúa." },
    detail: { title: "Inspecciona el marcador", description: "El detalle muestra puntuación, pausa e integridad. Pausar, reanudar y reiniciar son operaciones explícitas." },
    competitions: { title: "Configura competiciones", description: "Las competiciones necesitan rankings compatibles y límites claros. Son experimentales; verifica el overlay antes de usarlas en directo." },
  },
  pluginSetup: {
    catalog: { title: "Abre el plugin instalado", description: "Empieza en Instalados, no en Explorar, cuando configures un plugin ya confiable en esta máquina." },
    details: { title: "Revisa permisos y estado", description: "La cabecera muestra activación, versión y acciones del runtime. Revisa permisos antes de activarlo." },
    settings: { title: "Configura ajustes", description: "La pestaña Ajustes contiene valores específicos del plugin. Guarda primero cuando el plugin lo pida antes de probar." },
    connection: { title: "Conecta y prueba", description: "La pestaña Conexión gestiona credenciales externas y ofrece una acción de prueba cuando el plugin la proporciona." },
  },
  connectionRecovery: {
    form: { title: "Revisa creador y estado", description: "Empieza con el handle exacto y el estado actual. Offline, autenticación y sala no disponible requieren recuperaciones distintas." },
    explorer: { title: "Verifica la sala LIVE", description: "Actualiza la lista y confirma que la sala sea accesible. Puede estar offline, restringida o no disponible para invitados." },
    plugins: { title: "Revisa integraciones tras reconectar", description: "Las integraciones pueden ir detrás de una reconexión. Revisa su salud antes de asumir que fallaron automatizaciones u overlays." },
  },
  feedInspector: {
    controls: { title: "Abre el inspector", description: "Usa el control del inspector en la barra del feed. No pausa el LIVE ni cambia eventos guardados." },
    panel: { title: "Entiende el panel", description: "El inspector tiene pestañas de espectadores, detalles del evento y perfil. Su contenido sigue lo que selecciones." },
    viewers: { title: "Compara espectadores activos", description: "Cambia entre top y likes para ver quién impulsa la actividad de la sesión actual." },
    details: { title: "Inspecciona un evento", description: "Los detalles incluyen conteos, puntos, hora y evidencia del mensaje útiles para diagnosticar automatizaciones." },
    profile: { title: "Abre el perfil", description: "La pestaña de perfil resume al espectador y sus mensajes recientes sin exponer datos privados de la cuenta." },
  },
  backupRestore: {
    tools: { title: "Elige la fuente", description: "Las copias Cloud y los archivos locales son rutas de recuperación separadas. Elige la fuente antes de revisar qué cambiará." },
    list: { title: "Restaura deliberadamente", description: "Cada copia puede descargarse, restaurarse o eliminarse. Restaurar siempre muestra una revisión antes de aplicarse." },
    plan: { title: "Verifica el resultado", description: "Tras restaurar, revisa plan, perfiles, ajustes y automatizaciones clave. Una restauración correcta no revalida todas las credenciales externas." },
  },
} satisfies TourCatalog;
