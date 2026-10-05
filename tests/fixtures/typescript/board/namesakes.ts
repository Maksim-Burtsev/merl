export class Crowd {
  uid: number;
  nickname: string;
  permission: number;
  insightsEnabled: boolean;
  href: string;
  viewer: string;
}

export class Mailer {
  submit(url: string) {
    return url;
  }
}
