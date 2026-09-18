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
    this.close = this.close.bind(this);
  }

  close(): void {
    this.labels = [];
    this.audit = new AuditLog();
    void this.repo.deleteUser(this.posterId);
    this.log.deleteUser(this.posterId);
  }

  label(kind: string): string {
    switch (kind) {
      case "poster": {
        return String(this.posterId);
      }
      case "copy": return {
        posterId: 1,
        label(): string {
          return `${this.posterId}`;
        },
      }.label();
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
