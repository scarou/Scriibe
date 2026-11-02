import linux from '../../../logo/scriibe-logotype-linux.svg?url';
import linuxDark from '../../../logo/scriibe-logotype-linux-sombre.svg?url';
import macos from '../../../logo/scriibe-logotype-macos.svg?url';
import macosDark from '../../../logo/scriibe-logotype-macos-sombre.svg?url';
import windows from '../../../logo/scriibe-logotype-windows.svg?url';
import windowsDark from '../../../logo/scriibe-logotype-windows-sombre.svg?url';

const files = { macos: { light: macos, dark: macosDark }, windows: { light: windows, dark: windowsDark }, linux: { light: linux, dark: linuxDark } };

/** Le logotype (le mot complet) de la couleur du système, pour fond clair et pour fond sombre (`logo/LISEZMOI.md`). */
export function logotype(os: string | undefined): { light: string; dark: string } {
  return files[os as keyof typeof files] ?? files.windows;
}
