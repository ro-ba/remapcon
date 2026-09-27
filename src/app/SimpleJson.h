#pragma once

#include <cstdint>
#include <limits>
#include <map>
#include <string>
#include <vector>

namespace simple_json {
struct Value {
    enum class Type { Null, Boolean, Number, String, Array, Object } type = Type::Null;
    bool boolean = false;
    std::uint64_t number = 0;
    std::wstring string;
    std::vector<Value> array;
    std::map<std::wstring, Value> object;

    const Value* Get(const wchar_t* key) const {
        if (type != Type::Object) return nullptr;
        const auto found = object.find(key);
        return found == object.end() ? nullptr : &found->second;
    }
};

class Parser {
public:
    explicit Parser(const std::wstring& source) : source_(source) {}
    bool Parse(Value& value) {
        SkipSpace();
        if (!ReadValue(value, 0)) return false;
        SkipSpace();
        return offset_ == source_.size();
    }

private:
    const std::wstring& source_;
    size_t offset_ = 0;

    void SkipSpace() {
        while (offset_ < source_.size() && (source_[offset_] == L' ' ||
               source_[offset_] == L'\r' || source_[offset_] == L'\n' ||
               source_[offset_] == L'\t')) ++offset_;
    }
    bool Take(wchar_t ch) {
        SkipSpace();
        if (offset_ >= source_.size() || source_[offset_] != ch) return false;
        ++offset_;
        return true;
    }
    bool Literal(const wchar_t* value) {
        const std::wstring text(value);
        if (source_.compare(offset_, text.size(), text) != 0) return false;
        offset_ += text.size();
        return true;
    }
    bool Hex(unsigned& output) {
        if (source_.size() - offset_ < 4) return false;
        output = 0;
        for (int index = 0; index < 4; ++index) {
            const wchar_t ch = source_[offset_++];
            const unsigned digit = ch >= L'0' && ch <= L'9' ? ch - L'0' :
                ch >= L'a' && ch <= L'f' ? ch - L'a' + 10 :
                ch >= L'A' && ch <= L'F' ? ch - L'A' + 10 : 16;
            if (digit > 15) return false;
            output = (output << 4) | digit;
        }
        return true;
    }
    bool ReadString(std::wstring& output) {
        if (!Take(L'"')) return false;
        while (offset_ < source_.size()) {
            const wchar_t ch = source_[offset_++];
            if (ch == L'"') return true;
            if (ch < 32) return false;
            if (ch != L'\\') { output += ch; continue; }
            if (offset_ == source_.size()) return false;
            const wchar_t escape = source_[offset_++];
            switch (escape) {
            case L'"': output += L'"'; break;
            case L'\\': output += L'\\'; break;
            case L'/': output += L'/'; break;
            case L'b': output += L'\b'; break;
            case L'f': output += L'\f'; break;
            case L'n': output += L'\n'; break;
            case L'r': output += L'\r'; break;
            case L't': output += L'\t'; break;
            case L'u': {
                unsigned code = 0;
                if (!Hex(code)) return false;
                if (code >= 0xd800 && code <= 0xdbff) {
                    if (source_.size() - offset_ < 6 || source_.compare(offset_, 2, L"\\u") != 0)
                        return false;
                    offset_ += 2;
                    unsigned low = 0;
                    if (!Hex(low) || low < 0xdc00 || low > 0xdfff) return false;
                    output += static_cast<wchar_t>(code);
                    output += static_cast<wchar_t>(low);
                } else {
                    if (code >= 0xdc00 && code <= 0xdfff) return false;
                    output += static_cast<wchar_t>(code);
                }
                break;
            }
            default: return false;
            }
        }
        return false;
    }
    bool ReadValue(Value& output, unsigned depth) {
        if (depth > 32) return false;
        SkipSpace();
        if (offset_ == source_.size()) return false;
        const wchar_t ch = source_[offset_];
        if (ch == L'"') { output.type = Value::Type::String; return ReadString(output.string); }
        if (ch == L't') { output.type = Value::Type::Boolean; output.boolean = true; return Literal(L"true"); }
        if (ch == L'f') { output.type = Value::Type::Boolean; return Literal(L"false"); }
        if (ch == L'n') return Literal(L"null");
        if (ch == L'-' || (ch >= L'0' && ch <= L'9')) {
            if (ch == L'-') return false;
            output.type = Value::Type::Number;
            if (ch == L'0' && offset_ + 1 < source_.size() && source_[offset_ + 1] >= L'0' &&
                source_[offset_ + 1] <= L'9') return false;
            while (offset_ < source_.size() && source_[offset_] >= L'0' && source_[offset_] <= L'9') {
                const unsigned digit = source_[offset_++] - L'0';
                if (output.number > (std::numeric_limits<std::uint64_t>::max)() / 10 ||
                    (output.number == (std::numeric_limits<std::uint64_t>::max)() / 10 &&
                     digit > (std::numeric_limits<std::uint64_t>::max)() % 10)) return false;
                output.number = output.number * 10 + digit;
            }
            return true;
        }
        if (ch == L'[') {
            output.type = Value::Type::Array;
            ++offset_;
            if (Take(L']')) return true;
            do {
                Value element;
                if (!ReadValue(element, depth + 1)) return false;
                output.array.push_back(std::move(element));
                if (output.array.size() > 10000) return false;
                if (Take(L']')) return true;
            } while (Take(L','));
            return false;
        }
        if (ch == L'{') {
            output.type = Value::Type::Object;
            ++offset_;
            if (Take(L'}')) return true;
            do {
                std::wstring key;
                if (!ReadString(key) || !Take(L':')) return false;
                Value element;
                if (!ReadValue(element, depth + 1) ||
                    !output.object.emplace(std::move(key), std::move(element)).second) return false;
                if (output.object.size() > 10000) return false;
                if (Take(L'}')) return true;
            } while (Take(L','));
            return false;
        }
        return false;
    }
};
} // namespace simple_json
