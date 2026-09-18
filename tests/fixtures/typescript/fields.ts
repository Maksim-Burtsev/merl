import { AuditLog, UserRepository } from "./repos";

export class Base {
  audit = new AuditLog();
}

export class Issue extends Base {
  title: string;
  posterId = 0;
  labels: string[] = [];

  constructor(
    private repo: UserRepository,
    private log: AuditLog,
  ) {
    super();
    this.title = "";
  }

  close(): void {
    this.labels = [];
    void this.repo.deleteUser(this.posterId);
    this.log.deleteUser(this.posterId);
  }

  label(kind: string): string {
    switch (kind) {
      case "poster": {
        return String(this.posterId);
      }
      default: {
        return this.title;
      }
    }
  }
}

export interface Comment {
  posterId: number;
  body: string;
}

export function show(issue: Issue, comment: any): void {
  console.log(issue.posterId, issue.title, issue.audit);
  console.log(comment.posterId, comment.body);
}

export function tally(comments: any[]): number {
  let total = 0;
  const sums = {
    total: 0,
  };
  for (const comment of comments) {
    total += comment.total + sums.total;
  }
  return total;
}
