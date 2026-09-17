import { openSession, Session } from "./sessions";

export { openSession, Session };

export default function connect(): Session {
  return openSession();
}
