// noinspection JSXNamespaceValidation

import { JSX } from 'solid-js'
import { render as renderSolid } from 'solid-js/web'
import { BoxAttrs, PrimitiveAttrs } from 'universal'

export function createSpan(props: { children: JSX.Element } & PrimitiveAttrs): JSX.Element {
  const span = document.createElement('span')
  span.className = props.className ?? ''
  span.style.display = props.visible ? 'inline' : 'none'
  renderSolid(() => props.children, span)
  return span
}

export function createDiv(props: { children: JSX.Element } & BoxAttrs): JSX.Element {
  const div = document.createElement('div')
  div.className = props.className ?? ''
  div.style.display = props.visible ? 'flex' : 'none'
  div.style.flexDirection = props.direction ?? 'column'
  div.style.width = `${props.width}em` ?? '100%'
  div.style.height = `${props.height}em` ?? '100%'
  div.style.paddingLeft = `${props.paddingLeft}em` ?? '0'
  div.style.paddingRight = `${props.paddingRight}em` ?? '0'
  div.style.paddingTop = `${props.paddingTop}em` ?? '0'
  div.style.paddingBottom = `${props.paddingBottom}em` ?? '0'
  div.style.marginLeft = `${props.marginLeft}em` ?? '0'
  div.style.marginRight = `${props.marginRight}em` ?? '0'
  div.style.marginTop = `${props.marginTop}em` ?? '0'
  div.style.marginBottom = `${props.marginBottom}em` ?? '0'
  div.style.overflow = 'hidden'
  renderSolid(() => props.children, div)
  return div
}

export function createNewline(): JSX.Element {
  return document.createElement('br')
}
