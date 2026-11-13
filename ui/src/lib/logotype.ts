import linux from '../../../logo/scriibe-logotype-linux.svg?url';
import linuxDark from '../../../logo/scriibe-logotype-linux-sombre.svg?url';
import macos from '../../../logo/scriibe-logotype-macos.svg?url';
import macosDark from '../../../logo/scriibe-logotype-macos-sombre.svg?url';
import windows from '../../../logo/scriibe-logotype-windows.svg?url';
import windowsDark from '../../../logo/scriibe-logotype-windows-sombre.svg?url';
import linuxAnim from '../../../logo/anime/scriibe-logotype-linux.svg?url';
import linuxAnimDark from '../../../logo/anime/scriibe-logotype-linux-sombre.svg?url';
import macosAnim from '../../../logo/anime/scriibe-logotype-macos.svg?url';
import macosAnimDark from '../../../logo/anime/scriibe-logotype-macos-sombre.svg?url';
import windowsAnim from '../../../logo/anime/scriibe-logotype-windows.svg?url';
import windowsAnimDark from '../../../logo/anime/scriibe-logotype-windows-sombre.svg?url';

const files = { macos: { light: macos, dark: macosDark }, windows: { light: windows, dark: windowsDark }, linux: { light: linux, dark: linuxDark } };
const animated = {
  macos: { light: macosAnim, dark: macosAnimDark },
  windows: { light: windowsAnim, dark: windowsAnimDark },
  linux: { light: linuxAnim, dark: linuxAnimDark },
};

/** Le logotype (le mot complet) de la couleur du système, pour fond clair et pour fond sombre (`logo/LISEZMOI.md`). */
/** `animate` : le curseur clignote lentement (versions de `logo/anime/`, qui restent fixes si le système réduit les animations) ; sinon le logotype fixe. */
export function logotype(os: string | undefined, animate = true): { light: string; dark: string } {
  const set = animate ? animated : files;
  return set[os as keyof typeof set] ?? set.windows;
}
