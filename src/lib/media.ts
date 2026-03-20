/**
 * Rewrites Anki HTML content to use the anki-media:// protocol
 * for images and audio elements
 */

export function rewriteMediaUrls(html: string): string {
  if (!html) return html;
  
  let result = html;
  
  // Rewrite img src tags
  // Pattern: src="filename.ext" or src="path/to/filename.ext"
  result = result.replace(
    /src="([^"]+)"/g,
    (match, src) => {
      // Skip if already using a protocol
      if (src.includes('://') || src.startsWith('data:')) {
        return match;
      }
      // Encode the path and rewrite to anki-media://
      const encoded = encodeURIComponent(src);
      return `src="anki-media://${encoded}"`;
    }
  );
  
  // Rewrite audio src tags
  result = result.replace(
    /<audio([^>]*)src="([^"]+)"([^>]*)>/g,
    (match, prefix, src, suffix) => {
      // Skip if already using a protocol
      if (src.includes('://') || src.startsWith('data:')) {
        return match;
      }
      const encoded = encodeURIComponent(src);
      return `<audio${prefix}src="anki-media://${encoded}"${suffix}>`;
    }
  );
  
  // Also handle source tags inside audio elements
  result = result.replace(
    /<source([^>]*)src="([^"]+)"([^>]*)type="([^"]+)"([^>]*)>/g,
    (match, prefix, src, mid, type, suffix) => {
      // Skip if already using a protocol
      if (src.includes('://') || src.startsWith('data:')) {
        return match;
      }
      const encoded = encodeURIComponent(src);
      return `<source${prefix}src="anki-media://${encoded}"${mid}type="${type}"${suffix}>`;
    }
  );
  
  // Handle background images in style attributes
  result = result.replace(
    /background-image:\s*url\("([^"]+)"\)/g,
    (match, url) => {
      // Skip if already using a protocol
      if (url.includes('://') || url.startsWith('data:')) {
        return match;
      }
      const encoded = encodeURIComponent(url);
      return `background-image: url("anki-media://${encoded}")`;
    }
  );
  
  return result;
}

/**
 * Extracts all media filenames from HTML content
 */
export function extractMediaFiles(html: string): string[] {
  if (!html) return [];
  
  const mediaFiles: string[] = [];
  
  // Match img src
  const imgRegex = /src="([^"]+)"/g;
  let match;
  while ((match = imgRegex.exec(html)) !== null) {
    const src = match[1];
    if (!src.includes('://') && !src.startsWith('data:')) {
      mediaFiles.push(src);
    }
  }
  
  // Match audio src
  const audioRegex = /<audio[^>]*src="([^"]+)"/g;
  while ((match = audioRegex.exec(html)) !== null) {
    const src = match[1];
    if (!src.includes('://') && !src.startsWith('data:')) {
      mediaFiles.push(src);
    }
  }
  
  return mediaFiles;
}

/**
 * Fetches a media file from the backend and returns it as a data URL
 * This is used to load media files that are stored in the collection.media folder
 */
export async function fetchMediaFile(filename: string): Promise<string> {
  const { invoke } = await import("@tauri-apps/api/core");
  try {
    // Call the backend to get the media file as a base64 data URL
    const dataUrl = await invoke<string>("get_media_file", { filename });
    return dataUrl;
  } catch (error) {
    console.error("Failed to fetch media file:", filename, error);
    // Return a placeholder or empty string
    return "";
  }
}
