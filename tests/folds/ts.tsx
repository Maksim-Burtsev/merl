// Every construct `f` folds in TypeScript and TSX, and the cases that once broke it.
import { useState } from "react";  // f: 2-6
import type {  // f: 2-6
  Props,  // f: 2-6
  State,  // f: 2-6
} from "./types";  // f: 2-6

interface Point {  // f: 8-11
  x: number;  // f: 8-11
  y: number;  // f: 8-11
}  // f: 8-11

type Size =  // f: 13-15
  | "small"  // f: 13-15
  | "large";  // f: 13-15

enum Color {  // f: 17-20
  Red,  // f: 17-20
  Green,  // f: 17-20
}  // f: 17-20

@Entity({ name: "users" })
export class User {  // f: 23-29
  async load(  // f: 24-28
    id: string,  // f: 24-28
  ): Promise<User> {  // f: 26-28
    return fetchUser(id);  // f: 24-28
  }  // f: 24-28
}  // f: 23-29

export function Card({ title, body }: Props) {  // f: 31-54
  const [open, setOpen] = useState(false);  // f: 31-54
  const style = css`  // f: 31-54
    color: ${(props) =>  // f: 34-35
      props.color};  // f: 34-35
  `;  // f: 31-54
  return (  // f: 31-54
    <div  // f: 38-52
      className="card"  // f: 31-54
      onClick={() => {  // f: 40-42
        setOpen(!open);  // f: 40-42
      }}  // f: 40-42
    >  // f: 31-54
      <h1>{title}</h1>  // f: 31-54
      {open && (  // f: 31-54
        <p style={{  // f: 46-48
          margin: 0,  // f: 31-54
        }}>  // f: 31-54
          {body}  // f: 31-54
        </p>  // f: 31-54
      )}  // f: 31-54
    </div>  // f: 31-54
  );  // f: 31-54
}  // f: 31-54

export class History {  // f: 56-72
  commands(): Record<string, Command | Factory> {  // f: 57-61
    return {  // f: 58-60
      undo: () => undo,  // f: 57-61
    };  // f: 57-61
  }  // f: 57-61

  index(  // f: 63-71
    items: Item[],  // f: 63-71
  ): Promise<Map<string, Item>> {  // f: 65-71
    const m = new Map();  // f: 63-71
    for (const i of items) {  // f: 67-69
      m.set(i.id, i);  // f: 63-71
    }  // f: 63-71
    return m;  // f: 63-71
  }  // f: 63-71
}  // f: 56-72
