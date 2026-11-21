import type { TourCatalog } from './index.ts';
export default {
  controls: { next: 'Siguiente', previous: 'Anterior', done: 'Terminar guía', close: 'Cerrar guía', progress: '{{current}} de {{total}}' },
  connection: {
    form: { title: 'Elige un creador', description: 'Introduce su nombre y revisa el estado. La guía nunca conecta ni lee tu cookie de sesión.' },
    explorer: { title: 'Explora salas LIVE', description: 'Actualiza o elige una sala por tu cuenta. La conexión demo es simulada; el acceso nativo depende del host y de TikTok.' },
    plugins: { title: 'Comprueba integraciones', description: 'Revisa cada conexión instalada y su salud antes de habilitar efectos. Abrir la guía no contacta servicios de plugins.' },
  },
  automation: {
    profiles: { title: 'Elige el perfil de trabajo', description: 'Mantén un perfil para las reglas que quieras revisar. Cambiar o importar perfiles es una acción explícita de la aplicación.' },
    library: { title: 'Revisa eventos y acciones', description: 'Un evento vincula disparador, filtros y enfriamiento con acciones reutilizables. Conserva efectos nuevos desactivados hasta revisar su definición.' },
    templates: { title: 'Empieza con una plantilla', description: 'Abre Rule templates para previsualizar configuración reutilizable. Importar y activar requieren revisión propia; la guía no hace ninguna de las dos cosas.' },
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
} satisfies TourCatalog;
