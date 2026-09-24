/**
 * Profile pictures.
 *
 * Menlo's own icon is the default. The rest are the Recess gang, the same original
 * drawings that ship with Third Street Bookmarks — a nod to each character's
 * signature look rather than frames from the show. A picture you upload is cropped
 * and downscaled here, then stored by Rust beside the config.
 */

import menlo from "../assets/avatars/menlo.png";
import finster from "../assets/avatars/finster.svg";
import gretchen from "../assets/avatars/gretchen.svg";
import gus from "../assets/avatars/gus.svg";
import kingBob from "../assets/avatars/king-bob.svg";
import mikey from "../assets/avatars/mikey.svg";
import spinelli from "../assets/avatars/spinelli.svg";
import vince from "../assets/avatars/vince.svg";

export const DEFAULT_AVATAR = "menlo";
/** Matches `avatar::CUSTOM` in Rust. */
export const CUSTOM_AVATAR = "custom";

export interface Avatar {
  id: string;
  label: string;
  src: string;
}

export const AVATARS: Avatar[] = [
  { id: "menlo", label: "Menlo", src: menlo },
  { id: "spinelli", label: "Spinelli", src: spinelli },
  { id: "gretchen", label: "Gretchen", src: gretchen },
  { id: "gus", label: "Gus", src: gus },
  { id: "vince", label: "Vince", src: vince },
  { id: "mikey", label: "Mikey", src: mikey },
  { id: "king-bob", label: "King Bob", src: kingBob },
  { id: "finster", label: "Miss Finster", src: finster },
];

/** The picture to show for a setting, falling back to Menlo's own. */
export function avatarFor(id: string | undefined, uploaded: string | null | undefined): Avatar {
  if (id === CUSTOM_AVATAR && uploaded) {
    return { id: CUSTOM_AVATAR, label: "Your picture", src: uploaded };
  }
  return AVATARS.find((a) => a.id === id) ?? AVATARS[0];
}

/**
 * Downscale and centre-crop a chosen file to a square JPEG data URL.
 *
 * A phone photo is several megabytes and thousands of pixels wide; the picture never
 * renders above ~72px, so 256 keeps it sharp on a retina screen at a few tens of
 * kilobytes — small enough to hand across the IPC boundary in one string.
 */
export function squareImage(file: File, size = 256): Promise<string> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      const side = Math.min(img.naturalWidth, img.naturalHeight);
      const canvas = document.createElement("canvas");
      canvas.width = size;
      canvas.height = size;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        URL.revokeObjectURL(url);
        reject(new Error("This machine could not prepare the image"));
        return;
      }
      ctx.drawImage(
        img,
        (img.naturalWidth - side) / 2,
        (img.naturalHeight - side) / 2,
        side,
        side,
        0,
        0,
        size,
        size,
      );
      URL.revokeObjectURL(url);
      resolve(canvas.toDataURL("image/jpeg", 0.9));
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("That file is not an image this app can read"));
    };
    img.src = url;
  });
}
