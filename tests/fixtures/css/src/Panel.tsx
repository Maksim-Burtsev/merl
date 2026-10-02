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
