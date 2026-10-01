import './styles.css'

export const Button = () => <button className="btn-primary">OK</button>
//                                               ^ d: src/styles.css:4
// status: btn-primary: by name, 1 match
export const Ghost = () => <button className={'btn btn-ghost'}>OK</button>
//                                                  ^ d: picker src/styles.css:12, src/styles.css:13, src/styles.css:27
export const Main = () => <main id="main">x</main>
//                                   ^ d: src/styles.css:31
export const Title = () => <h1 className="card__title">t</h1>
//                                          ^ d: scss/buttons.scss:21
export const Nobody = () => <p className="nowhere">n</p>
//                                         ^ d: none
export function plain() {
  const primary = "btn-primary"
  return primary
  //      ^ d: src/Button.tsx:15
}
