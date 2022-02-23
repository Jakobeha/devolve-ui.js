declare module 'node-ansiparser' {
  export interface EventHandlers {
    inst_p: (str: string) => void
    inst_o: (str: string) => void
    inst_x: (flag: string) => void
    inst_c: (collected: string, params: string[], flag: string) => void
    inst_e: (collected: string, flag: string) => void
    inst_H: (collected: string, params: string[], flag: string) => void
    inst_P: (dcs: string) => void
    inst_U: () => void
  }

  declare class AnsiParser {
    constructor (events: EventHandlers): this;
    parse (string: string): void
  }
  export default AnsiParser
}
