#! /usr/bin/env node
// noinspection NodeCoreCodingAssistance

// region Boilerplate
const esbuild = require('esbuild')
const fs = require('fs')
const path = require('path')

function readdir (dir) {
  return new Promise((resolve, reject) => {
    fs.readdir(dir, (err, files) => {
      if (err) {
        reject(err)
      } else {
        resolve(files)
      }
    })
  })
}

/** Derived from https://stackoverflow.com/questions/46390733/recursively-read-a-directories-with-a-folder */
async function traverse (dir, accumulator = []) {
  for (const fileName of await readdir(dir)) {
    const filePath = path.resolve(dir, fileName)
    if (fs.statSync(filePath).isDirectory()) {
      await traverse(filePath, accumulator)
    } else {
      accumulator.push(filePath)
    }
  }
  return accumulator
}
// endregion

traverse('src').then(entryPoints => {
  // ES-modules unminified
  esbuild.build({
    entryPoints,
    sourcemap: true,
    minify: false,
    format: 'esm',
    outdir: 'out/src',
    watch: process.argv.includes('--watch')
  }).catch(() => {
    console.error('ES-modules unminified build failed')
  })
}).catch(err => {
  console.error('Traversing source files failed', err)
})
