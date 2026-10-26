# CP2077 Mod Launcher

[English](README.md) | [Türkçe](README.tr.md) | [Русский](README.ru.md) | [Español](README.es.md) | [Português (BR)](README.pt-BR.md) | [Français](README.fr.md) | [Polski](README.pl.md) | [Deutsch](README.de.md)

Un gestor de mods no oficial, hecho por la comunidad, para Cyberpunk 2077 en Windows. Versión 0.0.1 beta.

No lo hacen ni está afiliado a CD PROJEKT RED, Nexus Mods ni al equipo de NightCoop.

## Qué hace

- Instala mods desde archivos .zip, .rar y .7z. Suelta el archivo en la ventana o pulsa Instalar mod.
- Activa un mod copiando sus archivos a la carpeta del juego y lo desactiva borrando exactamente esos archivos. Los archivos sobrescritos se guardan y se devuelven.
- Avisa antes de que un mod sobrescriba un archivo de otro mod.
- Encuentra los mods que instalaste a mano y te deja importarlos.
- Reúne varias versiones de un mismo mod en una sola tarjeta, así se ve cuál es la actual.
- Comprueba e instala versiones de NightCoop desde la web oficial y verifica cada descarga con SHA-256.
- Explora y busca en Nexus Mods con filtros, etiquetas y desplazamiento infinito. Las cuentas gratuitas pueden instalar con enlaces nxm o desde la carpeta de descargas.
- Muestra los registros del juego, de los mods y del launcher, y los borra cuando se lo pides.
- Habla inglés, turco, ruso, español, portugués de Brasil, francés, polaco y alemán.
- Tiene temas del sistema, oscuro y claro.

## Instalación

1. Descarga el zip desde [Releases](https://github.com/This-null/cyberpunk-modloader/releases).
2. Descomprímelo y ejecuta el instalador.
3. Windows puede mostrar un aviso de SmartScreen porque el instalador no está firmado. Elige Más información y luego Ejecutar de todas formas.

## Privacidad

No se envía nada automáticamente. Los informes de error solo se guardan en tu ordenador. El launcher habla con la web de NightCoop y con Nexus Mods para listar y descargar mods. Las notas de versión van a Google Translate solo si lo permites en Ajustes. Tu clave de API de Nexus se queda en el Administrador de credenciales de Windows de tu ordenador.

## Compilar desde el código

Necesitas Node 20 o superior, pnpm y Rust estable.

```
pnpm install
pnpm tauri dev
pnpm tauri build
```

## Licencia

MIT, consulta [LICENSE](LICENSE). Los archivos RAR se descomprimen con el código de UnRAR a través del paquete unrar, que tiene su propia licencia.

Usa los mods bajo tu propia responsabilidad y haz copia de tus partidas antes.
