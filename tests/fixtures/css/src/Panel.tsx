import styles from './Button.module.css'
import { container as box, title } from './Card.module.scss'

export const Panel = () => <div className={styles.container}>x</div>
//                                                   ^ d: src/Button.module.css:1
// status: container: via import src/Button.module.css
export const Head = () => <h1 className={styles.title}>t</h1>
//                                               ^ d: src/Button.module.css:4
export const Wide = () => <div className={box}>w</div>
//                                         ^ d: src/Card.module.scss:4
// status: container: via import src/Card.module.scss
export const Missing = () => <h2 className={title}>m</h2>
//                                           ^ d: none
export const Gone = () => <p className={styles.nowhere}>n</p>
//                                              ^ d: none
// CSS Modules: a default import and a named one, each narrowed to its file (#590)
interface Theme {
  container: string
}
const theme = { styles: { title: 'c' } }
export const Nested = () => <div className={theme.styles.title}>n</div>
//                                                       ^ d: none
export function Shadow(styles: Theme) { return styles.container }
//                                                    ^ d: src/Panel.tsx:18
export const Arrow = (styles: Theme) => styles.container
//                                             ^ d: src/Panel.tsx:18
export const Boxed = (box: string) => <div className={box}>b</div>
//                                                    ^ d: src/Panel.tsx:27
//                    ^ d: src/Panel.tsx:27
import plain from './base.css'
export const Plain = () => <p className={plain.lede}>p</p>
//                                             ^ d: none
export function Indented() {
  return <div className={styles.container}>i</div>
//                              ^ d: src/Button.module.css:1
}
// A styles that is not the import, a chain past it and a stylesheet that is no module stay TypeScript's (#590)
