import type { ExperienceCopy } from '../types.ts';

export default {
  interactive: 'Ejemplo interactivo',
  fictional: 'Datos ficticios',
  tryIt: 'Probar este ejemplo',
  reset: 'Reiniciar ejemplo',
  openFull: 'Abrir demo completa',
  close: 'Cerrar ejemplo',
  preview: 'Vista previa',
  loading: 'Cargando la interfaz…',
  task: 'Tu siguiente paso',
  success: 'Qué debes comprobar',
  topicsTitle: 'Aprende con la aplicación delante',
  topicsDescription: 'Elige una tarea, prueba sus controles y comprueba el resultado antes de usarla en tu LIVE.',
  readGuide: 'Ver guía paso a paso',
  topics: {
    connection: {
      title: 'Conecta tu primer LIVE',
      description: 'Introduce un creador y reconoce los estados de conexión.',
      task: 'Escribe demo_creator y pulsa Conectar. En este ejemplo no necesitas una cookie.',
      result: 'El estado cambia a conectado. Abre el ejemplo del feed para practicar con eventos ficticios.',
    },
    automation: {
      title: 'Convierte eventos en acciones',
      description: 'Explora una regla de chat y revisa lo que ejecuta.',
      task: 'Revisa el evento de chat, la acción vinculada y su historial de ejecución.',
      result: 'Cada ejecución muestra su resultado. Los datos y efectos de esta demo son simulados.',
    },
    overlays: {
      title: 'Prepara tu escena en OBS',
      description: 'Edita un diseño y comprueba su vista previa.',
      task: 'Elige un widget, cambia sus propiedades y repite la vista previa.',
      result: 'El diseño refleja tus cambios. La recepción en OBS se comprueba después en la aplicación instalada.',
    },
    points: {
      title: 'Configura los puntos de tu audiencia',
      description: 'Revisa las reglas, los saldos y los niveles.',
      task: 'Abre las reglas de puntos y revisa las fuentes activadas antes de guardar.',
      result: 'La configuración se guarda solo en este ejemplo. Una política real se verifica con un evento nuevo.',
    },
  },
  otherTasks: {
    feed: {
      task: 'Pulsa Siguiente evento y selecciona el evento nuevo. Abre el perfil de su espectador.',
      result: 'El feed muestra el evento ficticio y sus detalles; el perfil identifica al espectador.',
    },
    analytics: {
      task: 'Revisa los totales, pulsa Siguiente evento y compara los valores de nuevo.',
      result: 'Los totales reflejan la actividad ficticia recibida durante esta demo.',
    },
    plugins: {
      task: 'Examina un plugin ficticio y prueba su instalación en la demo.',
      result: 'Su estado cambia dentro de la demo sin instalar ni conectar un servicio real.',
    },
    cloud: {
      task: 'Compara los valores local y Cloud de greeting y elige el cambio que hayas revisado.',
      result: 'La comparación refleja tu decisión dentro del escenario simulado.',
    },
    collaboration: {
      task: 'Revisa la solicitud ficticia y concede únicamente el acceso limitado que necesite.',
      result: 'La demo registra los permisos elegidos sin unir ningún dispositivo externo.',
    },
    settings: {
      task: 'Cambia una preferencia local y reinicia la demo para volver al estado inicial.',
      result: 'La preferencia cambia solo en esta instancia y se restablece al reiniciarla.',
    },
    diagnostics: {
      task: 'Selecciona el registro fallido, revisa sus detalles y reintenta la acción simulada.',
      result: 'Los detalles permiten identificar el fallo y comprobar el resultado del reintento.',
    },
  },
  previewLabels: {
    creator: 'Creador', event: 'Evento', action: 'Acción', output: 'Resultado',
    chat: 'Mensaje de chat', gift: 'Regalo', connected: 'Conectado', disconnected: 'Desconectado',
    message: '¡Hola, TikTools!',
  },
  connectionStates: {
    label: 'Situación de práctica', normal: 'Conexión correcta', offline: 'Creador sin LIVE', auth: 'Sesión no válida',
    offlineHelp: 'Comprueba que el creador esté transmitiendo y que el identificador sea correcto antes de volver a conectar.',
    authHelp: 'Si la sala requiere autenticación, utiliza una sesión autorizada reciente en el campo de cookie. No compartas su valor.',
  },
  releases: {
    stable: 'Estable', prerelease: 'Preliminar', notes: 'Notas de la versión', source: 'Código fuente',
    downloads: 'Descargas disponibles', noDownloads: 'No hay descargas publicadas para este sistema en esta versión.',
    checksums: 'Sumas de verificación', published: 'Publicada', snapshot: 'Datos consultados en GitHub',
    refresh: 'Consultar publicaciones actuales', all: 'Todos', highlights: 'Cambios destacados',
    unavailable: 'Consulta GitHub para comprobar las descargas disponibles.',
  },
} satisfies ExperienceCopy;
