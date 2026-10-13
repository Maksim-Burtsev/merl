// ledger.h: the accounts of a small shop
#pragma once

#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

#define LEDGER_VERSION 3

namespace shop::books {

enum class Kind : std::uint8_t { Debit, Credit };

struct Entry {
    std::string memo;
    long cents = 0;
    Kind kind{Kind::Debit};
    bool posted : 1;
};

template <typename T>
concept Amount = requires(T a) {
    { a + a } -> std::same_as<T>;
};

/// A ledger keeps entries in the order they were posted.
class Ledger final : public Base, private Audit {
public:
    using Map = std::map<std::string, std::vector<Entry>>;

    explicit Ledger(std::string name) noexcept;
    Ledger(const Ledger&) = delete;
    Ledger& operator=(Ledger&&) = default;
    virtual ~Ledger() override;

    [[nodiscard]] long total() const;
    std::optional<Entry> find(std::string_view memo) const;
    template <Amount A>
    A sum(const A& start) const;

    static constexpr int kLimit = 1'000;
    friend bool operator==(const Ledger& a, const Ledger& b);

protected:
    void post(Entry&& e);

private:
    std::string name_;
    Map accounts_;
    mutable int reads_ = 0;
    inline static thread_local int open_ = 0;
};

}  // namespace shop::books
