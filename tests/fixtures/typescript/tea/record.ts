import { Model as SequelizeModel } from "sequelize-typescript";

export class Record extends SequelizeModel {
  stamp = 0;
}

export class Revision extends Record {
  title = "";
}

type Dimension = {
  changed: "width" | "height";
};

export const sizes: Dimension[] = [];
