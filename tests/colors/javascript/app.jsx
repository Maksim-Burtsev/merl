import React from "react";
import PropTypes from "prop-types";

const MAX_ITEMS = 10;

export default function App({ items, user }) {
  const visible = items.slice(0, MAX_ITEMS);
  return (
    <main id="app" className={user ? "signed-in" : "guest"}>
      <h1>Hello, {user?.name ?? "stranger"}!</h1>
      <List.Group items={visible} onSelect={(item) => console.log(item)} />
      {visible.length === 0 ? <p>No items</p> : null}
      <a href="/about" target="_blank">
        About
      </a>
      <custom-element some-attr="x" />
    </main>
  );
}

App.propTypes = {
  items: PropTypes.array.isRequired,
  user: PropTypes.shape({ name: PropTypes.string }),
};
