
use ws_ugv_protocol::messages::*;
use ws_ugv_protocol::*;

use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tokio::io::BufReader;
use tokio::task;
use r2r::{QosProfile, Node, Publisher};

use r2r::geometry_msgs::msg::{Quaternion, Vector3};
use r2r::sensor_msgs::msg::Imu;

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


fn dispatch_base_data(basedata: BaseInfoData,  imu_publisher: & Publisher<Imu>)
{
    let msg = Imu {
        orientation: Quaternion {w: basedata.q0, x: basedata.q1, y: basedata.q2, z: basedata.q3},
        angular_velocity: Vector3 {x: basedata.gx, y: basedata.gy, z: basedata.gz},
        linear_acceleration: Vector3 {x: basedata.ax, y: basedata.ay, z: basedata.az},
        ..Default::default()
    };

    imu_publisher.publish(&msg).unwrap();
}

async fn ugv_loop(mut readport: & mut BufReader<SerialStream>, imu_publisher: & Publisher<Imu>)
{
    loop {
        let res = read_feedback(& mut readport).await;

        if res.is_err() {
            println!("Error {:?}", res);
            continue
        }

        match res.unwrap() {
            FeedbackMessage::IMU(imudata) => dispatch_imu_data(imudata),
            FeedbackMessage::BaseInfo(basedata) => dispatch_base_data(basedata, imu_publisher),
            FeedbackMessage::IMUOffset(imuoffset) => dispatch_imu_offset(imuoffset)
        };
    }
}

async fn app() {

    let ctx = r2r::Context::create().unwrap();
    let mut node = r2r::Node::create(ctx, "rust_bot", "ugv").unwrap();

    let imu_publisher =
        node.create_publisher::<r2r::sensor_msgs::msg::Imu>("/imu", QosProfile::default()).unwrap();
    //let mut timer = node.create_wall_timer(std::time::Duration::from_millis(1000)).unwrap();

    let (mut writeport, mut buf_readport) = construct_ugv_ports("/dev/serial0").await;

    ugv_loop(& mut buf_readport, & imu_publisher).await;

    let res = task::spawn_blocking(move ||ros_loop(node)).await;

    res.unwrap().await;
}
