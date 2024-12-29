cmake_minimum_required(VERSION 3.5)
project(ws_ugv_protocol)

find_package(ament_cmake REQUIRED)

if(NOT DEFINED CMAKE_SUPPRESS_DEVELOPER_WARNINGS)
     set(CMAKE_SUPPRESS_DEVELOPER_WARNINGS 1 CACHE INTERNAL "No dev warnings")
endif()

include(r2r_cargo.cmake)

# ros package dependencies
r2r_cargo(sensor_msgs
          geometry_msgs
          rcl                    # c ros2 api needed
          rcl_action             # as of r2r 0.1.0, action api needed
         )

# install binaries
if(WIN32)
  set(SUFFIX ".exe")
else()
  set(SUFFIX "")
endif()

install(PROGRAMS
  ${CMAKE_SOURCE_DIR}/target/colcon/${PROJECT_NAME}${SUFFIX}
  DESTINATION lib/${PROJECT_NAME}
)

# Needed for ros/colcon
ament_package()