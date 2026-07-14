// Glue for src/wasm.rs over @team-charls/charls-wasm (npm). One decoder, reused.
// The package resolves its own .wasm, so no URL is needed here.

import { createJpegLSDecoder } from '@team-charls/charls-wasm'

let decoder = null

export async function initCharls () {
  if (!decoder) decoder = await createJpegLSDecoder()
}

export function isInitialised () {
  return decoder !== null
}

export function decodeJpegLs (source) {
  if (!decoder) throw new Error('charls: init() was never awaited')
  // decode() returns a view over the wasm heap that the next decode overwrites.
  return new Uint8Array(decoder.decode(source))
}

/** Header only, no decode. */
export function frameInfo (source) {
  if (!decoder) throw new Error('charls: init() was never awaited')
  decoder.reset()
  decoder.setSourceBuffer(source)
  decoder.readHeader()
  const info = decoder.getFrameInfo()
  return [info.width, info.height, info.bitsPerSample, info.componentCount]
}
