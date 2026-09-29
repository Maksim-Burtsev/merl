type Props = { title: string };

export function card(props: Props): string {
  return props.title;
}

export function tally(o: { x: number[] }): number {
  const values = o.x;
  return values.length;
}
