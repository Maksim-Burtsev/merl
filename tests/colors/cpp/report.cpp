#include "ledger.h"

#include <algorithm>
#include <cstdio>
#include <iostream>

using namespace std::literals;
using shop::books::Ledger;

namespace {
constexpr auto kHeader = R"(memo,cents
)";
const char* kNames[] = {"cash", u8"bank", L"card"};
}  // namespace

Ledger::Ledger(std::string name) noexcept : name_(std::move(name)), reads_{0} {}

Ledger::~Ledger() = default;

long Ledger::total() const {
    ++reads_;
    long sum = 0;
    for (const auto& [key, entries] : accounts_) {
        for (auto const& e : entries) {
            sum += e.kind == Kind::Credit ? -e.cents : e.cents;
        }
    }
    return sum;
}

std::optional<Entry> Ledger::find(std::string_view memo) const {
    auto it = std::find_if(accounts_.begin(), accounts_.end(), [&memo](const auto& p) mutable {
        return p.first == memo;
    });
    if (it == accounts_.end() || it->second.empty()) {
        return std::nullopt;
    }
    return it->second.front();
}

template <typename A>
A Ledger::sum(const A& start) const {
    A acc = start;
    for (auto i = 0u; i < 10; i++) acc = acc + static_cast<A>(i);
    return acc;
}

void Ledger::post(Entry&& e) {
    try {
        if (e.cents < 0) throw std::invalid_argument("negative");
        accounts_[e.memo].push_back(std::move(e));
    } catch (const std::exception& err) {
        std::cerr << "post: " << err.what() << '\n';
    } catch (...) {
        throw;
    }
}

bool operator==(const Ledger& a, const Ledger& b) { return a.name_ == b.name_; }

class Report : public virtual Printer<Report, 2> {
public:
    void print(FILE* out) const override {
        std::fprintf(out, "%-10s %8.2f\n", name.c_str(), cents / 100.0);
        printf("%d entries, %s\n", count, kHeader);
    }

private:
    std::string name = "report"s;
    double cents = 0x1Fp3 + 1e-3 + 0b1010 + 017;
    int count = sizeof(Entry) + alignof(Entry);
    Report* self = this;
    std::unique_ptr<int[]> buf{new int[4]};
};

struct Point { int x, y; };

static_assert(sizeof(Point) == 8, "packed");

co_task<int> fetch(Channel& ch) {
    auto v = co_await ch.read();
    co_yield v;
    co_return v * 2;
}

int main(int argc, char** argv) {
    Ledger book{"main"};
    Point p{.x = 1, .y = 2};
    auto* raw = reinterpret_cast<char*>(&p);
    delete[] raw;
    if constexpr (Ledger::kLimit > 0) {
        goto done;
    }
done:
    std::cout << book.total() << std::endl;
    return argc > 1 ? EXIT_FAILURE : EXIT_SUCCESS;
}
