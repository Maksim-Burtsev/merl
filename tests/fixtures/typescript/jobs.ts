import connectToStore, { openSession } from "./store";
import * as sessions from "@/store/sessions";
import { Session as StoreSession } from "./store/sessions.js";
import { UserRepository } from "./repos";

export function nightly(repo: UserRepository): void {
  connectToStore();
  openSession();
  sessions.openSession().close();
  StoreSession.start();
  void repo;
}
