// A C++ function named like an Objective-C class method: a C++ call is never the method (#417).
void warmUp() {}

void heat() { warmUp(); }
//            ^ d: objc/warm.cc:2
