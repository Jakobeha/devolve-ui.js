import { BoxAttrs, Elements, PrimitiveAttrs } from 'universal'
import { For, Index, JSX, Match, Show, Switch } from 'solid-js'
import { createDiv, createNewline, createSpan } from 'web/createElements'

export const elements: Elements<JSX.Element> = {
  Text: (props: { children: JSX.Element } & PrimitiveAttrs): JSX.Element => createSpan(props),
  Box: (props: { children: JSX.Element } & BoxAttrs): JSX.Element => createDiv(props),
  Newline: (): JSX.Element => createNewline(),
  For,
  Index,
  Show,
  // @ts-expect-error
  Switch,
  // @ts-expect-error
  Match
}
