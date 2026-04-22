#pragma once

/// A simple stack container.
template<typename T>
class Stack {
private:
    std::vector<T> data_;

public:
    void push(const T& value) {
        data_.push_back(value);
    }

    bool empty() const {
        return data_.empty();
    }
};
