declare function props<T>(): T;

const given = props<{
  locale: string;
  //^ d: keys.ts:4
  count?: number;
  //^ d: keys.ts:6
}>();

const options = {
  locale: given.locale,
  //^ d: none
};

const locale = given.locale;
const count = options.locale.length;
console.log(locale, count);
//          ^ d: keys.ts:15

const keyedNest = {
  inner: {
    locale: given.locale } };
    //^ d: keys.ts:15; want none (#1291)

interface KeyedShape {
  locale: string;
  //^ d: keys.ts:26
}
