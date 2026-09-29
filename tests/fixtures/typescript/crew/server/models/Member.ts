// Headers prettier wrapped at `<` declare their type (#331).
class Member extends ParanoidModel<
  InferAttributes<Member>,
  Partial<InferCreationAttributes<Member>>
> {
  name: string;
}

export default Member;
