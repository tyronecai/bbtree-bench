cmake_minimum_required(VERSION 3.16)

project(bbtree_kv_bench_cpp LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(ABSL_BUILD_TESTING OFF CACHE BOOL "" FORCE)
set(ABSL_ENABLE_INSTALL OFF CACHE BOOL "" FORCE)
set(ABSL_USE_SYSTEM_INCLUDES ON CACHE BOOL "" FORCE)
set(BUILD_TESTING OFF CACHE BOOL "" FORCE)

add_subdirectory(
  "${CMAKE_CURRENT_LIST_DIR}/../abseil-cpp-20260817.0"
  "${CMAKE_CURRENT_BINARY_DIR}/abseil"
  EXCLUDE_FROM_ALL
)

add_executable(std_map_bench std_map_bench.cpp)
add_executable(absl_btree_bench absl_btree_bench.cpp)

target_link_libraries(absl_btree_bench PRIVATE absl::btree)

foreach(target IN ITEMS std_map_bench absl_btree_bench)
  target_compile_options(
    ${target}
    PRIVATE -O3 -DNDEBUG -flto -Wall -Wextra -Wpedantic -Wconversion -Werror
  )
  target_link_options(
    ${target}
    PRIVATE -flto -static-libstdc++ -static-libgcc
  )
  set_target_properties(
    ${target}
    PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/bin"
  )
endforeach()
