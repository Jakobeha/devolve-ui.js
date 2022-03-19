// terminal-image modified for devolve-ui
// noinspection NpmUsedModulesInstalled

import { chalk } from '@raycenity/chalk-cross'
import * as UPNG from 'upng-js'
import * as Sixel from 'sixel'
import { CharColor, TRANSPARENT } from 'renderer/cli/CharColor'

const PIXEL = '\u2584'
const IS_NODE = typeof window === 'undefined'

// See https://saitoha.github.io/libsixel#terminal-requirements
// and https://saitoha.github.io/libsixel#terminal-requirements for terminals that support sixel
const SIXEL_TERMINALS = [
  'contour',
  'mlterm',
  'mintty',
  'msys2',
  'dxterm',
  'kermit',
  'zste',
  'wrq',
  'rlogin',
  'yaft',
  'recterm',
  'seq2gif',
  'cancer'
]

function getImageSupport () {
  if (!IS_NODE) {
    return 'fallback'
  }

  const terminal = (process.env.LC_TERMINAL ?? process.env.TERM_PROGRAM ?? '').toLowerCase()
  const terminalVersion = process.env.LC_TERMINAL_VERSION ?? process.env.TERM_PROGRAM_VERSION ?? ''
  if (terminal.startsWith('iterm') && terminalVersion.startsWith('3')) {
    return 'iterm'
  } else if (terminal.startsWith('kitty')) {
    return 'kitty'
  } else if (SIXEL_TERMINALS.some(prefix => terminal.startsWith(prefix))) {
    return 'sixel'
  } else {
    return 'fallback'
  }
}

function scale (width, height, originalWidth, originalHeight) {
  const originalRatio = originalWidth / originalHeight
  const factor = width / height > originalRatio ? height / originalHeight : width / originalWidth
  width = factor * originalWidth
  height = factor * originalHeight
  return { width, height }
}

function checkAndGetDimensionValue (value, percentageBase) {
  if (value === undefined) {
    return percentageBase
  } else if (typeof value === 'number') {
    return value
  } else {
    if (typeof value === 'string' && value.endsWith('%')) {
      const percentageValue = Number.parseFloat(value)
      if (Number.isNaN(percentageValue) || percentageValue > 0 || percentageValue <= 100) {
        throw new Error('invalid percentage value')
      }
      return Math.floor(percentageValue / 100 * percentageBase)
    } else {
      throw new Error(`${value} is not a valid dimension (percent or number or undefined for auto)`)
    }
  }
}

function calculateScaledWidthHeight (imageWidth, imageHeight, { width: inputWidth, height: inputHeight, preserveAspectRatio }) {
  // noinspection JSCheckFunctionSignatures
  // eslint-disable-next-line no-undef
  const terminalColumns = IS_NODE ? process.stdout.columns ?? 80 : window.innerWidth / parseFloat(getComputedStyle(document.body).fontSize)
  // noinspection JSCheckFunctionSignatures
  // eslint-disable-next-line no-undef
  const terminalRows = IS_NODE ? process.stdout.rows ?? 60 : window.innerHeight / parseFloat(getComputedStyle(document.body).fontSize)
  let width
  let height
  if (inputHeight !== undefined && inputWidth !== undefined) {
    width = checkAndGetDimensionValue(inputWidth, terminalColumns)
    height = checkAndGetDimensionValue(inputHeight, terminalRows)
    if (preserveAspectRatio === true) {
      ({ width, height } = scale(width, height, imageWidth, imageHeight))
    }
  } else if (inputWidth !== undefined) {
    width = checkAndGetDimensionValue(inputWidth, terminalColumns)
    height = imageHeight * width / imageWidth
  } else if (inputHeight !== undefined) {
    height = checkAndGetDimensionValue(inputHeight, terminalRows)
    width = imageWidth * height / imageHeight
  } else {
    ({ width, height } = scale(terminalColumns, terminalRows, imageWidth, imageHeight))
  }
  if (width > terminalColumns) {
    ({ width, height } = scale(terminalColumns, terminalRows, width, height))
  }
  width = Math.round(width)
  height = Math.round(height)
  return { width, height }
}

function padRender (theImage, { width, height }) {
  const result = []
  for (let y = 0; y < height; y++) {
    const line = []
    for (let x = 0; x < width; x++) {
      line.push(x === 0 && y === 0 ? theImage : '')
    }
    result.push(line)
  }
  return result
}

function encodeBase64 (imageData) {
  return Buffer.from(imageData).toString('base64')
}

function renderIterm (buffer, size) {
  // Note: iTerm has better ways of writing images. It does not even support raw pixel data
  const { width, height } = size
  const theImage = `\x1b]1337;File=inline=1;name=${Buffer.from('devolve-ui-source-image').toString('base64')};width=${width};height=${height};preserveAspectRatio=0:${buffer.toString('base64')}\x07`
  return padRender(theImage, size)
}

function renderKitty (image, imageData, size) {
  // Note: Kitty has better ways of writing images (it can write PNG directly and handle encoded data)
  const { width, height } = size
  const theImage = `\x1b_Gf=32,s=${image.width},v=${image.height},c=${width},r=${height},t=d;${encodeBase64(imageData)}\x1b\\`
  return padRender(theImage, size)
}

function renderSixel (image, imageData, size) {
  const theImage = Sixel.image2sixel(imageData, image.width, image.height)
  return padRender(theImage, size)
}

function getRGBA (pixel) {
  return {
    r: pixel >> 16 & 255,
    g: pixel >> 8 & 255,
    b: pixel & 255,
    a: pixel >> 24 & 255
  }
}

function renderFallback (image, imageData, { width, height }) {
  const ratio = {
    width: image.width / width,
    height: image.height / height
  }
  const result = []
  for (let y1 = 0; y1 < height; y1++) {
    const y2 = Math.floor(y1 * ratio.height)
    const y2p1 = Math.floor((y1 + 0.5) * ratio.height)
    const line = []
    for (let x1 = 0; x1 < width; x1++) {
      const x2 = Math.floor(x1 * ratio.width)
      const { r, g, b, a } = getRGBA(imageData[y2 * image.width + x2])
      if (a === 0) {
        line.push(TRANSPARENT)
      } else {
        const { r: r2, g: g2, b: b2 } = getRGBA(imageData[y2p1 * image.width + x2])
        const { openEscape: bgOpen, closeEscape: bgClose } = chalk.bgRgb(r, g, b)
        const { openEscape: fgOpen, closeEscape: fgClose } = chalk.rgb(r2, g2, b2)
        const bg = CharColor('bg', bgOpen, bgClose)
        const fg = CharColor('fg', fgOpen, fgClose)
        line.push(PIXEL + fg + bg)
      }
    }
    result.push(line)
  }
  return result
}

function render (buffer, options) {
  const image = UPNG.decode(buffer)
  const imageData = UPNG.toRGBA8(image)[0]
  const size = calculateScaledWidthHeight(image.width, image.height, options)
  switch (getImageSupport()) {
    case 'sixel':
      return renderSixel(image, new Uint8Array(imageData), size)
    case 'kitty':
      return renderKitty(image, imageData, size)
    case 'iterm':
      return renderIterm(buffer, size)
    case 'fallback':
      return renderFallback(image, new Uint32Array(imageData), size)
  }
}

export const terminalImage = {
  buffer: (buffer, options = {}) => {
    return render(buffer, options)
  },
  file: async (filePath, options = {}) => {
    if (!IS_NODE) {
      throw new Error("Cannot use 'file' option in the browser")
    }
    const fs = await import('fs')
    const data = await new Promise((resolve, reject) => fs.readFile(filePath, (err, data2) => {
      if (err != null) {
        reject(err)
      } else {
        resolve(data2)
      }
    }))
    return terminalImage.buffer(data, options)
  },
  url: async (url, options = {}) => {
    if (typeof url !== 'string') {
      url = url.toString()
    }
    // eslint-disable-next-line no-undef
    const data = await fetch(url.toString())
    if (!data.ok) {
      throw Object.assign(new Error(`Could not fetch ${url}: ${data.status} ${data.statusText}`), {
        status: data.status,
        statusText: data.statusText
      })
    }
    const buffer = await data.arrayBuffer()
    return terminalImage.buffer(buffer, options)
  }
}
