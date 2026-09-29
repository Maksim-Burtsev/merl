#pragma once

class File {
 public:
  explicit File(const char* name) : filename_(name) {}
  //                                ^ d: include/shop/file.hh:11
  const char* Name() const { return filename_; }
  //                                ^ d: include/shop/file.hh:11

 private:
  const char* filename_;
};
