#pragma once

class File {
 public:
  explicit File(const char* name) : filename_(name) {}
  //                                ^ d: include/shop/file.hh:10
  const char* Name() const { return filename_; }

 private:
  const char* filename_;
};
