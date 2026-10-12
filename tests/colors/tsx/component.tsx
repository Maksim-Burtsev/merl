import React, { useState, type ReactNode } from "react";
import { Button } from "./Button";
import * as Icons from "./icons";

type Props = {
  title: string;
  children?: ReactNode;
  onClose: () => void;
};

export const DEFAULT_TITLE = "Untitled";

export function Card({ title = DEFAULT_TITLE, children, onClose }: Props) {
  const [open, setOpen] = useState<boolean>(false);
  const toggle = () => setOpen((value) => !value);

  return (
    <div className="card" data-open={open} aria-label="card">
      <header>
        <h1>{title}</h1>
        <Icons.Close size={16} onClick={onClose} />
      </header>
      {open && <section>{children}</section>}
      {/* a comment inside JSX */}
      <Button variant="primary" disabled={!open} onClick={toggle}>
        Toggle &amp; more
      </Button>
      <>
        <input type="text" value={title} readOnly />
        <br />
      </>
    </div>
  );
}

const identity = <T,>(value: T): T => value;

export default function List<T extends { id: string }>(props: { items: T[] }) {
  return (
    <ul>
      {props.items.map((item) => (
        <li key={item.id}>{identity(item.id)}</li>
      ))}
    </ul>
  );
}
