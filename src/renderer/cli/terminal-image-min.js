// noinspection NpmUsedModulesInstalled

import { chalk } from '@raycenity/chalk-cross'
import * as UPNG from 'upng-js'
import { CharColor, TRANSPARENT } from 'renderer/cli/CharColor'

const ROW_OFFSET = 2
const PIXEL = '\u2584'
const IS_NODE = typeof window === 'undefined'

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
  const terminalRows = IS_NODE ? process.stdout.rows - ROW_OFFSET : window.innerHeight / parseFloat(getComputedStyle(document.body).fontSize)
  let width
  let height
  if (inputHeight !== undefined && inputWidth !== undefined) {
    width = checkAndGetDimensionValue(inputWidth, terminalColumns)
    height = checkAndGetDimensionValue(inputHeight, terminalRows) * 2
    if (preserveAspectRatio !== false) {
      ({ width, height } = scale(width, height, imageWidth, imageHeight))
    }
  } else if (inputWidth !== undefined) {
    width = checkAndGetDimensionValue(inputWidth, terminalColumns)
    height = imageHeight * width / imageWidth
  } else if (inputHeight !== undefined) {
    height = checkAndGetDimensionValue(inputHeight, terminalRows) * 2
    width = imageWidth * height / imageHeight
  } else {
    ({ width, height } = scale(terminalColumns, terminalRows * 2, imageWidth, imageHeight))
  }
  if (width > terminalColumns) {
    ({ width, height } = scale(terminalColumns, terminalRows * 2, width, height))
  }
  width = Math.round(width)
  height = Math.round(height)
  return { width, height }
}

function getRGBA (pixel) {
  return {
    r: pixel >> 16 & 255,
    g: pixel >> 8 & 255,
    b: pixel & 255,
    a: pixel >> 24 & 255
  }
}

function render (buffer, options) {
  const image = UPNG.decode(buffer)
  const imageData = new Uint32Array(UPNG.toRGBA8(image)[0])
  const { width, height } = calculateScaledWidthHeight(image.width, image.height, options)
  const ratio = {
    width: image.width / width,
    height: image.height / height
  }
  const result = []
  for (let y1 = 0; y1 < height - 1; y1 += 2) {
    const y2 = Math.floor(y1 * ratio.height)
    const line = []
    for (let x1 = 0; x1 < width; x1++) {
      const x2 = Math.floor(x1 * ratio.width)
      const { r, g, b, a } = getRGBA(imageData[y2 * image.width + x2])
      if (a === 0) {
        line.push(TRANSPARENT)
      } else {
        const { r: r2, g: g2, b: b2 } = getRGBA(imageData[(y2 + 1) * image.width + x2])
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
