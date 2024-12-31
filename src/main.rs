
use ws_ugv_protocol::messages::*;
use ws_ugv_protocol::*;

use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tokio::io::BufReader;
use tokio::task;
use r2r::{QosProfile, Node, Publisher, Clock, ClockType};

use futures::stream::{StreamExt};
use futures::future;

use r2r::geometry_msgs::msg::{Quaternion, Vector3, Twist};
use r2r::sensor_msgs::msg::{Imu, JointState};
use r2r::std_msgs::msg::Header;

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let future = app();
    rt.block_on(future);
}


async fn construct_ugv_ports(device: & str) -> (SerialStream, BufReader<SerialStream>) {
    let mut readport = tokio_serial::new(device, 115200).open_native_async().expect("Failed to open port");
    readport.set_exclusive(false).unwrap();
    let buf_readport = BufReader::with_capacity(32000, readport);

    let mut writeport = tokio_serial::new(device, 115200).open_native_async().expect("Failed to open port");
    writeport.write_request_to_send(false).unwrap();
    writeport.write_data_terminal_ready(false).unwrap();

    (writeport, buf_readport)
}

async fn ros_loop(mut node: r2r::Node)
{
    loop {
        node.spin_once(std::time::Duration::from_millis(100));
    }
}

fn dispatch_imu_data(imudata: IMUData)
{

}

fn dispatch_imu_offset(imudata: IMUOffsetData)
{

}


fn dispatch_base_data(basedata: BaseInfoData,  imu_publisher: & Publisher<Imu>, joint_publisher: & Publisher<JointState>)
{
	let mut clock = Clock::create(ClockType::RosTime).unwrap();

    let msg = Imu {
        orientation: Quaternion {w: basedata.q0, x: basedata.q1, y: basedata.q2, z: basedata.q3},
        angular_velocity: Vector3 {x: basedata.gx, y: basedata.gy, z: basedata.gz},
        linear_acceleration: Vector3 {x: basedata.ax, y: basedata.ay, z: basedata.az},
        ..Default::default()
    };

    imu_publisher.publish(&msg).unwrap();

    // Only valid for older firmware pre gitsha bd829747abbabee202cf8296faf4ea70aaec7a30
    let left_pos = basedata.odl/0.0800;
    let right_pos = basedata.odr/0.0800;
    //let left_vel = 0.0800*basedata.l;
    //let right_vel = 0.0800*basedata.r;

	let cnow = clock.get_now().unwrap();
	let time = Clock::to_builtin_time(&cnow);


    let jmsg = JointState {
		header: Header {stamp: time, ..Default::default()},
        name: Vec::from(["front_left_wheel_joint", "front_right_wheel_joint", "mid_left_wheel_joint", "mid_right_wheel_joint", "rear_left_wheel_joint", "rear_right_wheel_joint"].map(String::from)),
        position: Vec::from([left_pos, right_pos, left_pos, right_pos, left_pos, right_pos]),
        ..Default::default()
    };

    joint_publisher.publish(&jmsg).unwrap();
}

async fn ugv_read_loop(mut readport: & mut BufReader<SerialStream>, imu_publisher: & Publisher<Imu>, joint_publisher: & Publisher<JointState>)
{
    loop {
        let res = read_feedback(& mut readport).await;

        if res.is_err() {
            println!("Error {:?}", res);
            continue
        }

        match res.unwrap() {
            FeedbackMessage::IMU(imudata) => dispatch_imu_data(imudata),
            FeedbackMessage::BaseInfo(basedata) => dispatch_base_data(basedata, imu_publisher, joint_publisher),
            FeedbackMessage::IMUOffset(imuoffset) => dispatch_imu_offset(imuoffset)
        };
    }
}

async fn ugv_write_loop(mut writeport: & mut SerialStream, cmd_vel_subscriber: &  impl StreamExt<Item = Twist>)
{
    cmd_vel_subscriber.for_each(|msg| {
        println!("got new msg: {:?}", msg.linear);
        future::ready(())
    }).await;
}

async fn app() {

    let ctx = r2r::Context::create().unwrap();
    let mut node = r2r::Node::create(ctx, "rust_bot", "ugv").unwrap();

    let imu_publisher =
        node.create_publisher::<Imu>("/imu", QosProfile::default()).unwrap();
    //let mut timer = node.create_wall_timer(std::time::Duration::from_millis(1000)).unwrap();

    let joint_publisher =
        node.create_publisher::<JointState>("/joint_states", QosProfile::default()).unwrap();

    let cmd_vel_subscriber =
        node.subscribe::<Twist>("/cmd_vel", QosProfile::default()).unwrap();

    let (mut writeport, mut buf_readport) = construct_ugv_ports("/dev/serial0").await;

    ugv_read_loop(& mut buf_readport, & imu_publisher, & joint_publisher).await;
    ugv_write_loop(& mut writeport, & cmd_vel_subscriber).await;

    let res = task::spawn_blocking(move ||ros_loop(node)).await;

    res.unwrap().await;
}
