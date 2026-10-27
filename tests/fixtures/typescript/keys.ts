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
    //^ d: none

interface KeyedShape {
  locale: string;
  //^ d: keys.ts:26
}

const deepNest = {
  "outer": {
    inner: {
      locale: given.locale } } };
      //^ d: none

let typedNest: {
  size: number;
  inner: {
    locale: string } };
    //^ d: keys.ts:39

const paramNest = {
  pick: (arg: {
    locale: string }) => arg };
    //^ d: keys.ts:44

const ternaryNest = {
  inner: count ? options : {
    locale: given.locale } };
    //^ d: none

interface NestedShape {
  inner: {
    locale: string } }
    //^ d: keys.ts:54

const defaulted = {
  inner: count ?? {
    locale: given.locale } };
    //^ d: none
