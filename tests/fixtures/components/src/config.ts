export type IssueInfo = {
  repoLink: string,
  repoId: number,
};

export interface AppWindow {
  config: {
    pageData: {
      repoLink?: string,
    },
  },
}

export class Converter {
  private readonly sortBy: string;
  constructor(sortBy: string) {
    this.sortBy = sortBy;
  }
  toRow() {
    return {
      options: {
        repoLink: this.sortBy,
      },
    };
  }
}

export interface ViewSettings {
  sortBy: string;
}

export function persisted<T>(value: T) {
  return { subscribe: (run: (v: T) => void) => run(value) };
}

export const viewSettings = persisted<ViewSettings>({ sortBy: "name" });

export const defaults = {
  orgs: [] as Array<{handle: string, full_slug: string}>,
};

export type Repo = {
  full_slug: string,
};
