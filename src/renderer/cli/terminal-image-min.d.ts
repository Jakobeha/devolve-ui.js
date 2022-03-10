import { VRender } from 'renderer/cli/VRender'

export declare type Percent = `${number}%`
export declare type Dimension = number | Percent | undefined
export interface ImageOptions {
  width?: Dimension
  height?: Dimension
  preserveAspectRatio?: boolean
}

export declare const terminalImage: {
  /**
   Display images in the terminal.
   Width and height be the percentage of the terminal window, the number of rows and/or columns, or undefined = 100%
   Please note that the image will always be scaled to fit the size of the terminal.
   By default, aspect ratio is always maintained. If you don't want to maintain aspect ratio, set preserveAspectRatio to false.
   */
  buffer: (buffer: ArrayBuffer, options?: ImageOptions) => VRender
  /**
   Display images in the terminal.
   Width and height be the percentage of the terminal window, the number of rows and/or columns, or undefined = 100%
   Please note that the image will always be scaled to fit the size of the terminal.
   By default, aspect ratio is always maintained. If you don't want to maintain aspect ratio, set preserveAspectRatio to false.
   */
  file: (filePath: string, options?: ImageOptions) => Promise<VRender>
  /**
   Display images in the terminal.
   Width and height be the percentage of the terminal window, the number of rows and/or columns, or undefined = 100%
   Please note that the image will always be scaled to fit the size of the terminal.
   By default, aspect ratio is always maintained. If you don't want to maintain aspect ratio, set preserveAspectRatio to false.
   */
  url: (url: URL | string, options?: ImageOptions) => Promise<VRender>
}
