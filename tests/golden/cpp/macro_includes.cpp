#include <iostream>
#include <memory>

auto ptr = std:: make_unique < int > (42);
cout << unique_ptr(ptr.get()) << std:: endl;
